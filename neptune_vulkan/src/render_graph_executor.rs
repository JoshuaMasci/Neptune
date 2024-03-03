use crate::descriptor_set::GpuBindingIndex;
use crate::device::{AshDevice, AshQueue};
use crate::image::vk_format_get_aspect_flags;
use crate::pipeline::Pipelines;
use crate::render_graph::{
    BufferBarrierSource, BufferOffset, CommandBuffer, CommandBufferDependency, CompiledRenderGraph,
    ComputeDispatch, DrawCommandDispatch, Framebuffer, ImageBarrierSource, ImageIndex, IndexType,
    RasterDrawCommand, RenderPassCommand, ShaderResourceUsage, Transfer,
};
use crate::resource_managers::{
    BufferResourceAccess, BufferTempResource, ImageTempResource, ResourceManager,
};
use crate::swapchain::{AcquiredSwapchainImage, SwapchainManager};
use crate::upload_queue::UploadPass;
use crate::{
    ComputePipelineHandle, RasterPipelineHandle, Sampler, SamplerHandle, SurfaceHandle, VulkanError,
};
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
        if self.next_index == 0 {
            return Ok(());
        }

        unsafe {
            self.device
                .core
                .wait_for_fences(&self.fences[0..self.next_index], true, timeout_ns)
        }
    }

    pub fn reset(&mut self) -> ash::prelude::VkResult<()> {
        if self.next_index == 0 {
            return Ok(());
        }

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

pub struct RenderGraphExecutor {
    device: Arc<AshDevice>,
    frame_contexts: Vec<FrameContext>,
    frame_index: usize,
}

impl RenderGraphExecutor {
    pub fn new(device: Arc<AshDevice>, frame_in_flight_count: u32) -> ash::prelude::VkResult<Self> {
        let mut frame_contexts = Vec::with_capacity(frame_in_flight_count as usize);
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

        //Upload Pass
        if let Some(upload_pass) = upload_pass {
            let upload_command_buffer = frame_context.graphics_command_pool.get()?;
            unsafe {
                self.device.core.begin_command_buffer(
                    upload_command_buffer,
                    &vk::CommandBufferBeginInfo::builder(),
                )?
            };

            let mut buffers =
                resource_manager.get_buffer_resources(&upload_pass.buffer_resources)?;
            let mut images =
                resource_manager.get_image_resources(&[], &upload_pass.image_resources)?;

            let mut resources = RenderGraphResources {
                buffers: &mut buffers,
                images: &mut images,
                persistent: resource_manager,
                pipelines,
            };

            record_command_buffer(
                &self.device,
                upload_command_buffer,
                &upload_pass.command_buffer,
                &mut resources,
            );

            unsafe {
                self.device.core.end_command_buffer(upload_command_buffer)?;

                let command_buffer_info = vk::CommandBufferSubmitInfo::builder()
                    .command_buffer(upload_command_buffer)
                    .build();
                self.device.core.queue_submit2(
                    self.device.graphics_queue.unwrap().handle,
                    &[vk::SubmitInfo2::builder()
                        .command_buffer_infos(&[command_buffer_info])
                        .build()],
                    vk::Fence::null(),
                )?;
            }
        }

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

        let mut staging_buffer_offset = 0;
        let mut staging_buffer = resource_manager.get_write_staging_buffer(
            render_graph
                .buffer_writes2
                .calc_needed_staging_size(&buffers),
        )?;

        // Write data to buffers
        let mut staging_buffer_copies: Vec<(BufferOffset, usize, usize)> = Vec::new();
        for buffer_write in render_graph.buffer_writes2.buffer_writes.iter() {
            if let Some(mapped_slice) = &mut buffers[buffer_write.buffer_offset.buffer]
                .mapped_slice
                .as_mut()
                .map(|mapped_slice| mapped_slice.slice_mut())
            {
                // If buffer is mapped write directly
                let write_start = buffer_write.buffer_offset.offset as usize;
                let write_end = write_start + buffer_write.write_size;
                buffer_write
                    .callback
                    .call(&mut mapped_slice[write_start..write_end]);
            } else {
                // Else write to staging buffer and copy to final buffer
                let staging_buffer = staging_buffer.as_mut().unwrap();
                let mapped_slice = staging_buffer.mapped_slice.as_mut().unwrap().slice_mut();
                let write_start = staging_buffer_offset;
                let write_end = write_start + buffer_write.write_size;
                buffer_write
                    .callback
                    .call(&mut mapped_slice[write_start..write_end]);
                staging_buffer_copies.push((
                    buffer_write.buffer_offset,
                    buffer_write.write_size,
                    staging_buffer_offset,
                ));
                staging_buffer_offset = write_end;
            }
        }

        //Buffer Writes/Reads
        resource_manager.write_buffers(&render_graph.buffer_writes);
        resource_manager.read_buffers(&render_graph.buffer_reads);

        let submit_queue = self.device.graphics_queue.unwrap().handle;

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

                //TODO: Properly schedule and barrier staging uploads
                for (target_buffer, write_size, src_offset) in staging_buffer_copies {
                    let src_buffer = staging_buffer.as_ref().unwrap();
                    let dst_buffer = &mut buffers[target_buffer.buffer];
                    dst_buffer.last_access = BufferResourceAccess::TransferWrite;
                    self.device.core.cmd_copy_buffer2(
                        vulkan_command_buffer,
                        &vk::CopyBufferInfo2::builder()
                            .src_buffer(src_buffer.buffer.handle)
                            .dst_buffer(dst_buffer.buffer.handle)
                            .regions(&[vk::BufferCopy2::builder()
                                .src_offset(src_offset as vk::DeviceSize)
                                .dst_offset(target_buffer.offset as vk::DeviceSize)
                                .size(write_size as vk::DeviceSize)
                                .build()]),
                    );
                }

                //Bind descriptor set
                {
                    let pipeline_bind_points = [
                        vk::PipelineBindPoint::COMPUTE,
                        vk::PipelineBindPoint::GRAPHICS,
                    ];
                    let set = resource_manager.descriptor_set.get_set();
                    for pipeline_bind_point in pipeline_bind_points {
                        self.device.core.cmd_bind_descriptor_sets(
                            vulkan_command_buffer,
                            pipeline_bind_point,
                            pipelines.layout,
                            0,
                            &[set],
                            &[],
                        );
                    }
                }

                if let Some(debug_util) = &self.device.instance.debug_utils {
                    debug_util.cmd_begin_label(
                        vulkan_command_buffer,
                        &format!("Command Buffer {}", command_buffer_index),
                        [1.0; 4],
                    );
                }

                //TODO: acquire resource ownership

                let mut resources = RenderGraphResources {
                    buffers: &mut buffers,
                    images: &mut images,
                    persistent: resource_manager,
                    pipelines,
                };
                record_command_buffer(
                    &self.device,
                    vulkan_command_buffer,
                    graph_command_buffer,
                    &mut resources,
                );

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
                        CommandBufferDependency::Swapchain { index, access } => {
                            Some((index, access))
                        }
                        _ => None,
                    })
                    .map(|(&swapchain_index, access)| {
                        let src = access.get_barrier_flags(true);
                        //Get last swapchain usages
                        vk::ImageMemoryBarrier2::builder()
                            .image(acquired_swapchain_images[swapchain_index].image.handle)
                            .old_layout(src.layout)
                            .src_stage_mask(src.stage_mask)
                            .src_access_mask(src.access_mask)
                            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                            .dst_stage_mask(vk::PipelineStageFlags2::NONE)
                            .dst_access_mask(vk::AccessFlags2::NONE)
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
                        CommandBufferDependency::Swapchain { index, access } => {
                            vk::SemaphoreSubmitInfo::builder()
                                .semaphore(acquired_swapchains[*index].image_ready_semaphore)
                                .stage_mask(access.get_barrier_flags(true).stage_mask)
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
                        CommandBufferDependency::Swapchain { index, access } => {
                            vk::SemaphoreSubmitInfo::builder()
                                .semaphore(acquired_swapchains[*index].present_ready_semaphore)
                                .stage_mask(access.get_barrier_flags(true).stage_mask)
                                .build()
                        }
                    })
                    .collect();

                // If the command buffer has no signal dependencies on other command buffers, that means it is a root node and should use a fence instead
                let command_buffer_done_fence = if command_buffer_dependency == 0 {
                    frame_context.fence_pool.get()?
                } else {
                    vk::Fence::null()
                };

                self.device.core.queue_submit2(
                    submit_queue,
                    &[vk::SubmitInfo2::builder()
                        .command_buffer_infos(&command_buffer_info)
                        .wait_semaphore_infos(&wait_semaphore_infos)
                        .signal_semaphore_infos(&signal_semaphore_infos)
                        .build()],
                    command_buffer_done_fence,
                )?;
            }
        }

        //Submit Swapchains
        if !acquired_swapchains.is_empty() {
            let mut swapchains = Vec::with_capacity(acquired_swapchains.len());
            let mut swapchain_indies = Vec::with_capacity(acquired_swapchains.len());
            let mut wait_semaphores = Vec::with_capacity(acquired_swapchains.len());
            for acquired_swapchain in acquired_swapchains.iter() {
                swapchains.push(acquired_swapchain.image.swapchain_handle);
                swapchain_indies.push(acquired_swapchain.image.image_index);
                wait_semaphores.push(acquired_swapchain.present_ready_semaphore);
            }
            unsafe {
                let _ = self.device.swapchain.queue_present(
                    submit_queue,
                    &vk::PresentInfoKHR::builder()
                        .swapchains(&swapchains)
                        .image_indices(&swapchain_indies)
                        .wait_semaphores(&wait_semaphores),
                );
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

    for (surface, _image_index) in swapchain_images.iter() {
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
    graph_resources: &mut RenderGraphResources,
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

        let buffer_barriers: Vec<vk::BufferMemoryBarrier2> = render_pass_set
            .buffer_barriers
            .iter()
            .map(|buffer_barrier| {
                let buffer = &graph_resources.buffers[buffer_barrier.index];
                let src = match buffer_barrier.src {
                    BufferBarrierSource::FirstUsage => buffer.last_access,
                    BufferBarrierSource::Precalculated(flags) => flags,
                }
                .get_barrier_flags();
                let dst = buffer_barrier.dst.get_barrier_flags();
                vk::BufferMemoryBarrier2::builder()
                    .buffer(buffer.buffer.handle)
                    .offset(0)
                    .size(vk::WHOLE_SIZE)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .src_stage_mask(src.stage_mask)
                    .src_access_mask(src.access_mask)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_stage_mask(dst.stage_mask)
                    .src_access_mask(dst.access_mask)
                    .build()
            })
            .collect();

        let image_barriers: Vec<vk::ImageMemoryBarrier2> = render_pass_set
            .image_barriers
            .iter()
            .map(|image_barrier| {
                let image = &graph_resources.images[image_barrier.index];
                let is_color = image.image.is_color();
                let src = match image_barrier.src {
                    ImageBarrierSource::FirstUsage => image.last_access.get_barrier_flags(is_color),
                    ImageBarrierSource::Precalculated(flags) => flags.get_barrier_flags(is_color),
                };
                let dst = image_barrier.dst.get_barrier_flags(is_color);
                vk::ImageMemoryBarrier2::builder()
                    .image(image.image.handle)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk_format_get_aspect_flags(image.image.format),
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .old_layout(src.layout)
                    .src_stage_mask(src.stage_mask)
                    .src_access_mask(src.access_mask)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .new_layout(dst.layout)
                    .dst_stage_mask(dst.stage_mask)
                    .dst_access_mask(dst.access_mask)
                    .build()
            })
            .collect();

        unsafe {
            device.core.cmd_pipeline_barrier2(
                vulkan_command_buffer,
                &vk::DependencyInfo::builder()
                    .memory_barriers(&render_pass_set.memory_barriers)
                    .buffer_memory_barriers(&buffer_barriers)
                    .image_memory_barriers(&image_barriers)
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

            if let Some(render_pass_command) = &render_pass.command {
                match render_pass_command {
                    RenderPassCommand::Transfer { transfers } => {
                        record_transfer_pass(
                            device,
                            vulkan_command_buffer,
                            graph_resources,
                            transfers,
                        );
                    }
                    RenderPassCommand::Compute {
                        pipeline,
                        resources,
                        dispatch,
                    } => record_compute_pass(
                        device,
                        vulkan_command_buffer,
                        graph_resources,
                        *pipeline,
                        resources,
                        dispatch,
                    ),
                    RenderPassCommand::Raster {
                        framebuffer,
                        draw_commands,
                    } => record_raster_pass(
                        device,
                        vulkan_command_buffer,
                        graph_resources,
                        framebuffer,
                        draw_commands,
                    ),
                }
            }

            if let Some(debug_util) = &device.instance.debug_utils {
                debug_util.cmd_end_label(vulkan_command_buffer);
            }
        }

        if let Some(debug_util) = &device.instance.debug_utils {
            debug_util.cmd_end_label(vulkan_command_buffer);
        }
    }
}

pub fn record_transfer_pass(
    device: &AshDevice,
    command_buffer: vk::CommandBuffer,
    graph_resources: &RenderGraphResources,
    transfers: &[Transfer],
) {
    for transfer in transfers.iter() {
        match transfer {
            Transfer::BufferToBuffer {
                src,
                dst,
                copy_size,
            } => {
                let src_buffer = &graph_resources.buffers[src.buffer].buffer;
                let dst_buffer = &graph_resources.buffers[dst.buffer].buffer;
                unsafe {
                    device.core.cmd_copy_buffer2(
                        command_buffer,
                        &vk::CopyBufferInfo2::builder()
                            .src_buffer(src_buffer.handle)
                            .dst_buffer(dst_buffer.handle)
                            .regions(&[vk::BufferCopy2::builder()
                                .src_offset(src.offset as vk::DeviceSize)
                                .dst_offset(dst.offset as vk::DeviceSize)
                                .size(*copy_size as vk::DeviceSize)
                                .build()]),
                    );
                }
            }
            Transfer::BufferToImage {
                src,
                dst,
                copy_size,
            } => {
                let src_buffer = &graph_resources.buffers[src.buffer].buffer;
                let dst_image = &graph_resources.images[dst.image].image;
                unsafe {
                    device.core.cmd_copy_buffer_to_image2(
                        command_buffer,
                        &vk::CopyBufferToImageInfo2::builder()
                            .src_buffer(src_buffer.handle)
                            .dst_image(dst_image.handle)
                            .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                            .regions(&[vk::BufferImageCopy2::builder()
                                .buffer_row_length(src.row_length.unwrap_or_default())
                                .buffer_image_height(src.row_height.unwrap_or_default())
                                .buffer_offset(src.offset)
                                .image_offset(vk::Offset3D {
                                    x: dst.offset[0] as i32,
                                    y: dst.offset[1] as i32,
                                    z: 0,
                                })
                                .image_extent(vk::Extent3D {
                                    width: copy_size[0],
                                    height: copy_size[1],
                                    depth: 1,
                                })
                                .image_subresource(vk::ImageSubresourceLayers {
                                    aspect_mask: vk_format_get_aspect_flags(dst_image.format),
                                    mip_level: 0,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                })
                                .build()]),
                    );
                }
            }
            Transfer::ImageToBuffer {
                src,
                dst,
                copy_size,
            } => {
                let src_image = &graph_resources.images[src.image].image;
                let dst_buffer = &graph_resources.buffers[dst.buffer].buffer;
                unsafe {
                    device.core.cmd_copy_image_to_buffer2(
                        command_buffer,
                        &vk::CopyImageToBufferInfo2::builder()
                            .src_image(src_image.handle)
                            .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                            .dst_buffer(dst_buffer.handle)
                            .regions(&[vk::BufferImageCopy2::builder()
                                .buffer_row_length(dst.row_length.unwrap_or_default())
                                .buffer_image_height(dst.row_height.unwrap_or_default())
                                .buffer_offset(dst.offset)
                                .image_offset(vk::Offset3D {
                                    x: src.offset[0] as i32,
                                    y: src.offset[1] as i32,
                                    z: 0,
                                })
                                .image_extent(vk::Extent3D {
                                    width: copy_size[0],
                                    height: copy_size[1],
                                    depth: 1,
                                })
                                .image_subresource(vk::ImageSubresourceLayers {
                                    aspect_mask: vk_format_get_aspect_flags(src_image.format),
                                    mip_level: 0,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                })
                                .build()]),
                    );
                }
            }
            Transfer::ImageToImage {
                src,
                dst,
                copy_size,
            } => {
                let src_image = &graph_resources.images[src.image].image;
                let dst_image = &graph_resources.images[dst.image].image;

                unsafe {
                    device.core.cmd_copy_image2(
                        command_buffer,
                        &vk::CopyImageInfo2::builder()
                            .src_image(src_image.handle)
                            .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                            .dst_image(dst_image.handle)
                            .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                            .regions(&[vk::ImageCopy2::builder()
                                .src_offset(vk::Offset3D {
                                    x: src.offset[0] as i32,
                                    y: src.offset[1] as i32,
                                    z: 0,
                                })
                                .dst_offset(vk::Offset3D {
                                    x: dst.offset[0] as i32,
                                    y: dst.offset[1] as i32,
                                    z: 0,
                                })
                                .extent(vk::Extent3D {
                                    width: copy_size[0],
                                    height: copy_size[1],
                                    depth: 1,
                                })
                                .src_subresource(vk::ImageSubresourceLayers {
                                    aspect_mask: vk_format_get_aspect_flags(src_image.format),
                                    mip_level: 0,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                })
                                .dst_subresource(vk::ImageSubresourceLayers {
                                    aspect_mask: vk_format_get_aspect_flags(dst_image.format),
                                    mip_level: 0,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                })
                                .build()]),
                    )
                }
            }
        }
    }
}

pub fn record_compute_pass(
    device: &AshDevice,
    command_buffer: vk::CommandBuffer,
    graph_resources: &RenderGraphResources,
    pipeline: ComputePipelineHandle,
    resources: &[ShaderResourceUsage],
    dispatch: &ComputeDispatch,
) {
    record_shader_resources(device, command_buffer, graph_resources, resources);

    unsafe {
        device.core.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            graph_resources.get_compute_pipeline(pipeline),
        );
        match dispatch {
            ComputeDispatch::Size(size) => {
                device
                    .core
                    .cmd_dispatch(command_buffer, size[0], size[1], size[2]);
            }
            ComputeDispatch::Indirect(buffer) => {
                device.core.cmd_dispatch_indirect(
                    command_buffer,
                    graph_resources.buffers[buffer.buffer].buffer.handle,
                    buffer.offset as vk::DeviceSize,
                );
            }
        }
    }
}

pub fn record_raster_pass(
    device: &AshDevice,
    command_buffer: vk::CommandBuffer,
    graph_resources: &RenderGraphResources,
    framebuffer: &Framebuffer,
    draw_commands: &[RasterDrawCommand],
) {
    //Begin Rendering
    {
        let mut rendering_info_builder = vk::RenderingInfo::builder().layer_count(1);

        let mut extent = None;
        let mut color_attachments = Vec::new();

        for color_attachment in framebuffer.color_attachments.iter() {
            let image = graph_resources.images[color_attachment.image].image;

            if let Some(extent) = extent {
                if extent != image.size {
                    panic!("Framebuffer color attachment extent does not match");
                }
            } else {
                extent = Some(image.size);
            }

            let color_clear = color_attachment.clear.map(|color| vk::ClearValue {
                color: vk::ClearColorValue { float32: color },
            });

            color_attachments.push(
                vk::RenderingAttachmentInfo::builder()
                    .image_view(image.view)
                    .image_layout(vk::ImageLayout::GENERAL)
                    .load_op(if color_clear.is_some() {
                        vk::AttachmentLoadOp::CLEAR
                    } else {
                        vk::AttachmentLoadOp::LOAD
                    })
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(color_clear.unwrap_or_default())
                    .build(),
            );
        }

        rendering_info_builder = rendering_info_builder.color_attachments(&color_attachments);

        let depth_stencil_attachment_info: vk::RenderingAttachmentInfo;
        if let Some(depth_stencil_image) = &framebuffer.depth_stencil_attachment {
            let image = graph_resources.images[depth_stencil_image.image].image;

            if let Some(extent) = extent {
                if extent != image.size {
                    panic!("Framebuffer depth stencil attachment extent does not match");
                }
            } else {
                extent = Some(image.size);
            }

            let color_clear = depth_stencil_image
                .clear
                .map(|depth_stencil| vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: depth_stencil.0,
                        stencil: depth_stencil.1,
                    },
                });

            depth_stencil_attachment_info = vk::RenderingAttachmentInfo::builder()
                .image_view(image.view)
                .image_layout(vk::ImageLayout::GENERAL)
                .load_op(if color_clear.is_some() {
                    vk::AttachmentLoadOp::CLEAR
                } else {
                    vk::AttachmentLoadOp::LOAD
                })
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(color_clear.unwrap_or_default())
                .build();

            rendering_info_builder =
                rendering_info_builder.depth_attachment(&depth_stencil_attachment_info);
        }

        let extent = extent.expect("Framebuffer has no attachments");

        let render_area = vk::Rect2D {
            offset: vk::Offset2D::default(),
            extent,
        };

        rendering_info_builder = rendering_info_builder
            .color_attachments(&color_attachments)
            .render_area(render_area);

        unsafe {
            device
                .core
                .cmd_begin_rendering(command_buffer, &rendering_info_builder);

            device.core.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: extent.width as f32,
                    height: extent.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );

            device
                .core
                .cmd_set_scissor(command_buffer, 0, &[render_area])
        }
    }

    //Draw calls
    for draw_call in draw_commands {
        //Bind Pipeline
        unsafe {
            device.core.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                graph_resources.get_raster_pipeline(draw_call.pipeline),
            );
        }

        //Bind Vertex Buffers
        if !draw_call.vertex_buffers.is_empty() {
            let mut vertex_buffers: Vec<vk::Buffer> =
                Vec::with_capacity(draw_call.vertex_buffers.len());
            for vertex_buffer in draw_call.vertex_buffers.iter() {
                vertex_buffers.push(graph_resources.buffers[vertex_buffer.buffer].buffer.handle);
            }

            let vertex_offset: Vec<vk::DeviceSize> = draw_call
                .vertex_buffers
                .iter()
                .map(|buffer| buffer.offset as vk::DeviceSize)
                .collect();

            unsafe {
                device.core.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &vertex_buffers,
                    &vertex_offset,
                );
            }
        }

        //Push Resource
        record_shader_resources(
            device,
            command_buffer,
            graph_resources,
            &draw_call.resources,
        );

        //Dispatch
        unsafe {
            match &draw_call.dispatch {
                DrawCommandDispatch::Draw {
                    vertices,
                    instances,
                } => device.core.cmd_draw(
                    command_buffer,
                    vertices.len() as u32,
                    instances.len() as u32,
                    vertices.start,
                    instances.start,
                ),
                DrawCommandDispatch::DrawIndexed {
                    base_vertex,
                    indices,
                    instances,
                    index_buffer,
                    index_type,
                } => {
                    device.core.cmd_bind_index_buffer(
                        command_buffer,
                        graph_resources.buffers[index_buffer.buffer].buffer.handle,
                        index_buffer.offset as vk::DeviceSize,
                        match index_type {
                            IndexType::U16 => vk::IndexType::UINT16,
                            IndexType::U32 => vk::IndexType::UINT32,
                        },
                    );

                    device.core.cmd_draw_indexed(
                        command_buffer,
                        indices.len() as u32,
                        instances.len() as u32,
                        indices.start,
                        *base_vertex,
                        instances.start,
                    );
                }
                DrawCommandDispatch::DrawIndirect {
                    indirect_buffer: buffer,
                    draw_count,
                    stride,
                } => device.core.cmd_draw_indirect(
                    command_buffer,
                    graph_resources.buffers[buffer.buffer].buffer.handle,
                    buffer.offset as vk::DeviceSize,
                    *draw_count,
                    *stride,
                ),
                DrawCommandDispatch::DrawIndirectIndexed {
                    indirect_buffer: buffer,
                    draw_count,
                    stride,
                    index_buffer,
                    index_type,
                } => {
                    device.core.cmd_bind_index_buffer(
                        command_buffer,
                        graph_resources.buffers[index_buffer.buffer].buffer.handle,
                        index_buffer.offset as vk::DeviceSize,
                        match index_type {
                            IndexType::U16 => vk::IndexType::UINT16,
                            IndexType::U32 => vk::IndexType::UINT32,
                        },
                    );
                    device.core.cmd_draw_indexed_indirect(
                        command_buffer,
                        graph_resources.buffers[buffer.buffer].buffer.handle,
                        buffer.offset as vk::DeviceSize,
                        *draw_count,
                        *stride,
                    );
                }
            }
        }
    }

    //End Rendering
    unsafe {
        device.core.cmd_end_rendering(command_buffer);
    }
}

fn record_shader_resources(
    device: &AshDevice,
    command_buffer: vk::CommandBuffer,
    graph_resources: &RenderGraphResources,
    resources: &[ShaderResourceUsage],
) {
    let mut push_bindings: Vec<GpuBindingIndex> = Vec::with_capacity(resources.len());

    for resource in resources.iter() {
        push_bindings.push(match resource {
            ShaderResourceUsage::StorageBuffer { buffer, .. } => graph_resources.buffers[*buffer]
                .buffer
                .storage_binding
                .expect("Buffer not bound as storage buffer"),
            ShaderResourceUsage::StorageImage { image, .. } => graph_resources.images[*image]
                .image
                .storage_binding
                .expect("Image not bound as storage image"),
            ShaderResourceUsage::SampledImage(image) => graph_resources.images[*image]
                .image
                .sampled_binding
                .expect("Image not bound as sampled image"),
            ShaderResourceUsage::Sampler(handle) => graph_resources
                .get_sampler(*handle)
                .binding
                .as_ref()
                .expect("Sampler is not bound")
                .index(),
        });
    }

    let push_data_bytes: Vec<u8> = push_bindings
        .drain(..)
        .flat_map(|binding| binding.to_bytes())
        .collect();

    unsafe {
        device.core.cmd_push_constants(
            command_buffer,
            graph_resources.get_pipeline_layout(),
            vk::ShaderStageFlags::ALL,
            0,
            &push_data_bytes,
        );
    }
}

pub struct RenderGraphResources<'a> {
    pub(crate) buffers: &'a mut [BufferTempResource],
    pub(crate) images: &'a mut [ImageTempResource],
    pub(crate) persistent: &'a mut ResourceManager,
    pub(crate) pipelines: &'a Pipelines,
}

impl<'a> RenderGraphResources<'a> {
    pub(crate) fn get_sampler(&self, resource: SamplerHandle) -> Arc<Sampler> {
        self.persistent
            .get_sampler(resource.0)
            .expect("Invalid Sampler Key")
    }

    pub(crate) fn get_compute_pipeline(&self, pipeline: ComputePipelineHandle) -> vk::Pipeline {
        self.pipelines.compute.get(pipeline.0).unwrap().handle
    }

    pub fn get_raster_pipeline(&self, pipeline: RasterPipelineHandle) -> vk::Pipeline {
        self.pipelines.raster.get(pipeline.0).unwrap().handle
    }

    pub fn get_pipeline_layout(&self) -> vk::PipelineLayout {
        self.pipelines.layout
    }
}
