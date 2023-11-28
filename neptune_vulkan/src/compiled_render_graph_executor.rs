use crate::device::{AshDevice, AshQueue};
use crate::pipeline::Pipelines;
use crate::render_graph::{
    CommandBuffer, CommandBufferDependency, CompiledRenderGraph, ImageIndex,
};

use crate::instance::SurfaceList;
use crate::render_graph_executor::RenderGraphResources;
use crate::resource_managers::ResourceManager;
use crate::swapchain::{AcquiredSwapchainImage, SwapchainManager};
use crate::upload_queue::UploadPass;
use crate::{SurfaceHandle, VulkanError};
use ash::vk;
use log::info;
use std::sync::Arc;

// Render Graph Executor Evolution
// 0. Whole pipeline barriers between passes, no image layout changes (only general layout), no pass order changes, no dead-code culling (DONE!)
// 1. Specific pipeline barriers between passes with image layout changes, no pass order changes, no dead-code culling
// 2. Whole graph evaluation with pass reordering and dead code culling
// 3. Multi-Queue execution
// 4. Sub resource tracking. Allowing image levels/layers and buffer regions to be transition and accessed in parallel

struct AshCommandPool {
    device: Arc<AshDevice>,
    handle: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    next_index: usize,
}

impl AshCommandPool {
    pub fn new(
        device: Arc<AshDevice>,
        queue: AshQueue,
        capacity: u32,
    ) -> ash::prelude::VkResult<Self> {
        let handle = unsafe {
            device.core.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(queue.family_index)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .build(),
                None,
            )
        }?;

        let command_buffers = unsafe {
            device.core.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(handle)
                    .command_buffer_count(capacity),
            )
        }?;

        Ok(Self {
            device,
            handle,
            command_buffers,
            next_index: 0,
        })
    }

    pub fn get(&mut self) -> ash::prelude::VkResult<vk::CommandBuffer> {
        if self.next_index >= self.command_buffers.len() {
            let mut new_command_buffers = unsafe {
                self.device.core.allocate_command_buffers(
                    &vk::CommandBufferAllocateInfo::builder()
                        .command_pool(self.handle)
                        .command_buffer_count(self.command_buffers.len().max(2) as u32),
                )?
            };
            self.command_buffers.append(&mut new_command_buffers);
        }

        let command_buffer = self.command_buffers[self.next_index];
        self.next_index += 1;
        Ok(command_buffer)
    }

    pub fn reset(&mut self) -> ash::prelude::VkResult<()> {
        self.next_index = 0;
        unsafe {
            self.device
                .core
                .reset_command_pool(self.handle, vk::CommandPoolResetFlags::empty())
        }
    }
}

impl Drop for AshCommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.core.destroy_command_pool(self.handle, None);
        }
    }
}

struct AshSemaphorePool {
    device: Arc<AshDevice>,
    semaphores: Vec<vk::Semaphore>,
    next_index: usize,
}

impl AshSemaphorePool {
    pub fn new(device: Arc<AshDevice>) -> Self {
        Self {
            device,
            semaphores: Vec::new(),
            next_index: 0,
        }
    }

    pub fn get(&mut self) -> ash::prelude::VkResult<vk::Semaphore> {
        if self.next_index >= self.semaphores.len() {
            unsafe {
                self.semaphores.push(
                    self.device
                        .core
                        .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?,
                );
            }
        }

        let semaphore = self.semaphores[self.next_index];
        self.next_index += 1;
        Ok(semaphore)
    }

    pub fn get_vec(&mut self, count: usize) -> ash::prelude::VkResult<Vec<vk::Semaphore>> {
        let mut vec = Vec::with_capacity(count);
        for _ in 0..count {
            vec.push(self.get()?);
        }
        Ok(vec)
    }

    pub fn reset(&mut self) {
        self.next_index = 0;
    }
}

impl Drop for AshSemaphorePool {
    fn drop(&mut self) {
        for semaphore in self.semaphores.drain(..) {
            unsafe {
                self.device.core.destroy_semaphore(semaphore, None);
            }
        }
    }
}

struct AshFencePool {
    device: Arc<AshDevice>,
    fences: Vec<vk::Fence>,
    next_index: usize,
}

impl AshFencePool {
    pub fn new(device: Arc<AshDevice>) -> Self {
        Self {
            device,
            fences: Vec::new(),
            next_index: 0,
        }
    }

    pub fn get(&mut self) -> ash::prelude::VkResult<vk::Fence> {
        if self.next_index >= self.fences.len() {
            unsafe {
                self.fences.push(
                    self.device
                        .core
                        .create_fence(&vk::FenceCreateInfo::default(), None)?,
                );
            }
        }

        let fence = self.fences[self.next_index];
        self.next_index += 1;
        Ok(fence)
    }

    pub fn wait_for_all(&self, timeout_ns: u64) -> ash::prelude::VkResult<()> {
        unsafe {
            self.device
                .core
                .wait_for_fences(&self.fences[0..self.next_index], true, timeout_ns)
        }
    }

    pub fn reset(&mut self) -> ash::prelude::VkResult<()> {
        unsafe {
            self.device
                .core
                .reset_fences(&self.fences[0..self.next_index])?;
        }
        self.next_index = 0;
        Ok(())
    }
}

impl Drop for AshFencePool {
    fn drop(&mut self) {
        for fence in self.fences.drain(..) {
            unsafe {
                self.device.core.destroy_fence(fence, None);
            }
        }
    }
}

struct FrameContext {
    graphics_command_pool: AshCommandPool,
    async_compute_command_pool: Option<AshCommandPool>,
    async_transfer_command_pool: Option<AshCommandPool>,
    semaphore_pool: AshSemaphorePool,
    fence_pool: AshFencePool,
}

impl FrameContext {
    pub fn new(device: Arc<AshDevice>) -> ash::prelude::VkResult<Self> {
        Ok(Self {
            graphics_command_pool: AshCommandPool::new(
                device.clone(),
                device.graphics_queue.expect("Requires a graphics queue"),
                8,
            )?,
            async_compute_command_pool: match device.compute_queue {
                None => None,
                Some(queue) => Some(AshCommandPool::new(device.clone(), queue, 4)?),
            },
            async_transfer_command_pool: match device.transfer_queue {
                None => None,
                Some(queue) => Some(AshCommandPool::new(device.clone(), queue, 4)?),
            },
            semaphore_pool: AshSemaphorePool::new(device.clone()),
            fence_pool: AshFencePool::new(device),
        })
    }

    pub fn wait_and_reset(&mut self, timeout_ns: u64) -> ash::prelude::VkResult<()> {
        self.fence_pool.wait_for_all(timeout_ns)?;
        self.fence_pool.reset()?;
        self.semaphore_pool.reset();

        self.graphics_command_pool.reset()?;
        if let Some(command_pool) = &mut self.async_compute_command_pool {
            command_pool.reset()?;
        }

        if let Some(command_pool) = &mut self.async_transfer_command_pool {
            command_pool.reset()?;
        }

        Ok(())
    }
}

pub struct CompiledRenderGraphExecutor {
    device: Arc<AshDevice>,
    frame_contexts: Vec<FrameContext>,
    frame_index: usize,
}

impl CompiledRenderGraphExecutor {
    pub fn new(
        device: Arc<AshDevice>,
        frame_in_flight_count: usize,
    ) -> ash::prelude::VkResult<Self> {
        let mut frame_contexts = Vec::with_capacity(frame_in_flight_count);
        for _ in 0..frame_contexts.capacity() {
            frame_contexts.push(FrameContext::new(device.clone())?)
        }
        Ok(Self {
            device,
            frame_contexts,
            frame_index: 0,
        })
    }

    pub(crate) fn submit_frame(
        &mut self,
        resource_manager: &mut ResourceManager,
        swapchain_manager: &mut SwapchainManager,
        pipelines: &Pipelines,
        upload_pass: Option<UploadPass>,
        render_graph: &CompiledRenderGraph,
    ) -> Result<(), VulkanError> {
        const TIMEOUT_NS: u64 = std::time::Duration::from_secs(2).as_nanos() as u64;
        self.frame_index = (self.frame_index + 1) % self.frame_contexts.len();

        let frame_context = &mut self.frame_contexts[self.frame_index];
        frame_context.wait_and_reset(TIMEOUT_NS)?;
        resource_manager.flush_frame();

        let command_buffer_dependency_semaphores = allocate_command_buffer_semaphores(
            &mut frame_context.semaphore_pool,
            &render_graph.command_buffers,
        )?;

        let acquired_swapchains = acquire_swapchain_images(
            &mut frame_context.semaphore_pool,
            swapchain_manager,
            &render_graph.swapchain_images,
        )?;
        let acquired_swapchain_images: Vec<AcquiredSwapchainImage> = acquired_swapchains
            .iter()
            .map(|swapchain| swapchain.image.clone())
            .collect();
        let mut buffers = resource_manager.get_buffer_resources(&render_graph.buffer_resources)?;
        let mut images = resource_manager
            .get_image_resources(&acquired_swapchain_images, &render_graph.image_resources)?;
        let mut resources = RenderGraphResources {
            buffers: &mut buffers,
            images: &mut images,
            persistent: resource_manager,
            pipelines,
        };

        for (command_buffer_index, graph_command_buffer) in
            render_graph.command_buffers.iter().enumerate()
        {
            //TODO: multi-queue
            let vulkan_command_buffer = frame_context.graphics_command_pool.get()?;

            unsafe {
                self.device.core.begin_command_buffer(
                    vulkan_command_buffer,
                    &vk::CommandBufferBeginInfo::builder(),
                )?;

                if let Some(debug_util) = &self.device.instance.debug_utils {
                    debug_util.cmd_begin_label(
                        vulkan_command_buffer,
                        &format!("Command Buffer {}", command_buffer_index),
                        [1.0; 4],
                    );
                }

                //TODO: acquire resource ownership

                record_command_buffer(&self.device, vulkan_command_buffer, graph_command_buffer);

                //TODO: release resource ownership

                if let Some(debug_util) = &self.device.instance.debug_utils {
                    debug_util.cmd_end_label(vulkan_command_buffer);
                }

                const SWAPCHAIN_SUBRESOURCE_RANGE: vk::ImageSubresourceRange =
                    vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    };

                let swapchain_transitions: Vec<vk::ImageMemoryBarrier2> = graph_command_buffer
                    .command_buffer_signal_dependencies
                    .iter()
                    .filter_map(|signal_dependency| match signal_dependency {
                        CommandBufferDependency::Swapchain { index, .. } => Some(index),
                        _ => None,
                    })
                    .map(|&swapchain_index| {
                        //Get last swapchain usages
                        let swapchain_image_resource_index =
                            render_graph.swapchain_images[swapchain_index].1;
                        let src_swapchain_barrier = &images[swapchain_image_resource_index]
                            .last_access
                            .get_barrier_flags(true); //Swapchain is always a color image

                        vk::ImageMemoryBarrier2::builder()
                            .image(acquired_swapchain_images[swapchain_index].image.handle)
                            .old_layout(src_swapchain_barrier.layout)
                            .src_stage_mask(src_swapchain_barrier.stage_mask)
                            .src_access_mask(src_swapchain_barrier.access_mask)
                            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                            .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                            .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
                            .subresource_range(SWAPCHAIN_SUBRESOURCE_RANGE)
                            .build()
                    })
                    .collect();

                if !swapchain_transitions.is_empty() {
                    self.device.core.cmd_pipeline_barrier2(
                        vulkan_command_buffer,
                        &vk::DependencyInfo::builder()
                            .image_memory_barriers(&swapchain_transitions)
                            .build(),
                    );
                }

                self.device.core.end_command_buffer(vulkan_command_buffer)?;
            }

            unsafe {
                let command_buffer_info = [vk::CommandBufferSubmitInfo::builder()
                    .command_buffer(vulkan_command_buffer)
                    .build()];

                let wait_semaphore_infos: Vec<vk::SemaphoreSubmitInfo> = graph_command_buffer
                    .command_buffer_wait_dependencies
                    .iter()
                    .map(|dependency| match dependency {
                        CommandBufferDependency::CommandBuffer {
                            command_buffer_index,
                            dependency_index,
                            stage_mask,
                            ..
                        } => vk::SemaphoreSubmitInfo::builder()
                            .semaphore(
                                command_buffer_dependency_semaphores[*command_buffer_index]
                                    [*dependency_index],
                            )
                            .stage_mask(*stage_mask)
                            .build(),
                        CommandBufferDependency::Swapchain { index, stage_mask } => {
                            vk::SemaphoreSubmitInfo::builder()
                                .semaphore(acquired_swapchains[*index].image_ready_semaphore)
                                .stage_mask(*stage_mask)
                                .build()
                        }
                    })
                    .collect();

                let mut command_buffer_dependency: u32 = 0;
                let signal_semaphore_infos: Vec<vk::SemaphoreSubmitInfo> = graph_command_buffer
                    .command_buffer_signal_dependencies
                    .iter()
                    .map(|dependency| match dependency {
                        CommandBufferDependency::CommandBuffer {
                            command_buffer_index,
                            dependency_index,
                            stage_mask,
                            ..
                        } => {
                            command_buffer_dependency += 1;
                            vk::SemaphoreSubmitInfo::builder()
                                .semaphore(
                                    command_buffer_dependency_semaphores[*command_buffer_index]
                                        [*dependency_index],
                                )
                                .stage_mask(*stage_mask)
                                .build()
                        }
                        CommandBufferDependency::Swapchain { index, stage_mask } => {
                            vk::SemaphoreSubmitInfo::builder()
                                .semaphore(acquired_swapchains[*index].present_ready_semaphore)
                                .stage_mask(*stage_mask)
                                .build()
                        }
                    })
                    .collect();

                // If the command buffer has no signal dependencies on other command buffers, that means it is a root node and should use a fence instead
                let command_buffer_done_fence = if command_buffer_dependency != 0 {
                    frame_context.fence_pool.get()?
                } else {
                    vk::Fence::null()
                };

                self.device.core.queue_submit2(
                    self.device.graphics_queue.unwrap().handle,
                    &[vk::SubmitInfo2::builder()
                        .command_buffer_infos(&command_buffer_info)
                        .wait_semaphore_infos(&wait_semaphore_infos)
                        .signal_semaphore_infos(&signal_semaphore_infos)
                        .build()],
                    command_buffer_done_fence,
                )?;
            }
        }

        Ok(())
    }
}

fn allocate_command_buffer_semaphores(
    semaphore_pool: &mut AshSemaphorePool,
    command_buffers: &[CommandBuffer],
) -> ash::prelude::VkResult<Vec<Vec<vk::Semaphore>>> {
    let mut vec = Vec::with_capacity(command_buffers.len());
    for command_buffer in command_buffers {
        vec.push(semaphore_pool.get_vec(command_buffer.command_buffer_signal_dependencies.len())?);
    }
    Ok(vec)
}

struct AcquiredSwapchain {
    image: AcquiredSwapchainImage,
    image_ready_semaphore: vk::Semaphore,
    present_ready_semaphore: vk::Semaphore,
}

fn acquire_swapchain_images(
    semaphore_pool: &mut AshSemaphorePool,
    swapchain_manager: &mut SwapchainManager,
    swapchain_images: &[(SurfaceHandle, ImageIndex)],
) -> ash::prelude::VkResult<Vec<AcquiredSwapchain>> {
    let mut acquire_swapchains = Vec::with_capacity(swapchain_images.len());

    for (surface, image_index) in swapchain_images.iter() {
        let swapchain = swapchain_manager
            .get(*surface)
            .expect("Failed to find swapchain");
        let image_ready_semaphore = semaphore_pool.get()?;
        let present_ready_semaphore = semaphore_pool.get()?;

        let mut swapchain_result: ash::prelude::VkResult<(AcquiredSwapchainImage, bool)> =
            swapchain.acquire_next_image(image_ready_semaphore);

        while let Err(vk::Result::ERROR_OUT_OF_DATE_KHR) = &swapchain_result {
            info!("Swapchain Out of Data, Rebuilding");
            swapchain.rebuild()?;
            swapchain_result = swapchain.acquire_next_image(image_ready_semaphore);
        }
        let image = swapchain_result.unwrap().0;

        acquire_swapchains.push(AcquiredSwapchain {
            image,
            image_ready_semaphore,
            present_ready_semaphore,
        });
    }

    Ok(acquire_swapchains)
}

fn record_command_buffer(
    device: &AshDevice,
    vulkan_command_buffer: vk::CommandBuffer,
    graph_command_buffer: &CommandBuffer,
) {
    for (render_pass_set_index, render_pass_set) in
        graph_command_buffer.render_pass_sets.iter().enumerate()
    {
        if let Some(debug_util) = &device.instance.debug_utils {
            debug_util.cmd_begin_label(
                vulkan_command_buffer,
                &format!("RenderPass Set {}", render_pass_set_index),
                [1.0; 4],
            );
        }

        //TODO: Buffer and Image Barriers
        unsafe {
            device.core.cmd_pipeline_barrier2(
                vulkan_command_buffer,
                &vk::DependencyInfo::builder()
                    .memory_barriers(&render_pass_set.memory_barriers)
                    .build(),
            );
        }

        for render_pass in render_pass_set.render_passes.iter() {
            if let Some(debug_util) = &device.instance.debug_utils {
                debug_util.cmd_begin_label(
                    vulkan_command_buffer,
                    &render_pass.label_name,
                    render_pass.label_color,
                );
            }

            //TODO: record passes

            if let Some(debug_util) = &device.instance.debug_utils {
                debug_util.cmd_end_label(vulkan_command_buffer);
            }
        }

        if let Some(debug_util) = &device.instance.debug_utils {
            debug_util.cmd_end_label(vulkan_command_buffer);
        }
    }
}
