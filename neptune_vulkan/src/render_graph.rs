use crate::{AshDevice, BufferKey, ImageKey, PersistentResourceManager, SwapchainManager};
use ash::vk;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct BufferAccess {
    write: bool, //TODO: calculate this from stage+access?
    stage: vk::PipelineStageFlags2,
    access: vk::AccessFlags2,
}

#[derive(Clone, Debug)]
pub struct ImageAccess {
    pub write: bool, //TODO: calculate this from stage+access+layout?
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
    pub layout: vk::ImageLayout,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum BufferResource {
    Persistent(BufferKey),
    Transient(usize),
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum ImageResource {
    Persistent(ImageKey),
    Transient(usize),
    Swapchain(usize),
}

pub type BuildCommandFn = dyn Fn(&AshDevice, vk::CommandBuffer, &mut RenderGraphResources);

pub struct ColorAttachment {
    pub image: ImageResource,
    pub clear: Option<[f32; 4]>,
}

impl ColorAttachment {
    pub fn new(image: ImageResource) -> Self {
        Self { image, clear: None }
    }

    pub fn new_clear(image: ImageResource, clear: [f32; 4]) -> Self {
        Self {
            image,
            clear: Some(clear),
        }
    }
}

pub struct DepthStencilAttachment {
    pub image: ImageResource,
    pub clear: Option<(f32, u32)>,
}

impl DepthStencilAttachment {
    pub fn new(image: ImageResource) -> Self {
        Self { image, clear: None }
    }

    pub fn new_clear(image: ImageResource, clear: (f32, u32)) -> Self {
        Self {
            image,
            clear: Some(clear),
        }
    }
}

#[derive(Default)]
pub struct Framebuffer {
    pub color_attachments: Vec<ColorAttachment>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
    pub input_attachments: Vec<ImageResource>,
}

#[derive(Default)]
pub struct RenderPass {
    pub name: String,
    pub queue: vk::Queue,
    pub buffer_usages: HashMap<BufferResource, BufferAccess>,
    pub image_usages: HashMap<ImageResource, ImageAccess>,
    pub framebuffer: Option<Framebuffer>,
    pub build_cmd_fn: Option<Box<BuildCommandFn>>,
}

#[derive(Debug, Clone)]
pub struct TransientBufferDesc {
    size: vk::DeviceSize,
    memory_location: gpu_allocator::MemoryLocation,
}

#[derive(Debug, Clone)]
pub struct TransientImageDesc {
    extent: vk::Extent2D,
    format: vk::Format,
    memory_location: gpu_allocator::MemoryLocation,
}

#[derive(Default)]
pub struct RenderGraph {
    pub transient_buffers: Vec<TransientBufferDesc>,
    pub transient_images: Vec<TransientImageDesc>,
    pub swapchain_images: Vec<vk::SurfaceKHR>,
    pub passes: Vec<RenderPass>,
}

impl RenderGraph {
    pub fn create_transient_buffer(&mut self, desc: TransientBufferDesc) -> BufferResource {
        let index = self.transient_buffers.len();
        self.transient_buffers.push(desc);
        BufferResource::Transient(index)
    }

    pub fn create_transient_image(&mut self, desc: TransientImageDesc) -> ImageResource {
        let index = self.transient_images.len();
        self.transient_images.push(desc);
        ImageResource::Transient(index)
    }

    pub fn acquire_swapchain_image(&mut self, surface: vk::SurfaceKHR) -> ImageResource {
        let index = self.swapchain_images.len();
        self.swapchain_images.push(surface);
        ImageResource::Swapchain(index)
    }

    pub fn add_pass(&mut self, pass: RenderPass) {
        self.passes.push(pass);
    }
}

pub struct RenderGraphResources<'a> {
    pub persistent: &'a mut PersistentResourceManager,
    // &mut TransientResourceManager,
    pub swapchain: &'a mut SwapchainManager,
}

use crate::device::AshQueue;
use log::info;
use std::sync::Arc;

pub struct BasicRenderGraphExecutor {
    device: Arc<AshDevice>,
    queue: AshQueue,

    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,

    swapchain_semaphores: Vec<(vk::Semaphore, vk::Semaphore)>,
    frame_done_fence: vk::Fence,
}

impl BasicRenderGraphExecutor {
    pub fn new(device: Arc<AshDevice>, device_queue_index: usize) -> ash::prelude::VkResult<Self> {
        let queue = device.queues[device_queue_index].clone();

        let command_pool = unsafe {
            device.core.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(queue.family_index)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .build(),
                None,
            )
        }?;

        let command_buffer = unsafe {
            device.core.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(command_pool)
                    .command_buffer_count(1),
            )
        }?[0];

        let frame_done_fence = unsafe {
            device.core.create_fence(
                &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )
        }?;

        let swapchain_semaphores = vec![unsafe {
            (
                device
                    .core
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?,
                device
                    .core
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?,
            )
        }];

        Ok(Self {
            device,
            queue,
            command_pool,
            command_buffer,
            swapchain_semaphores,
            frame_done_fence,
        })
    }

    pub fn execute_graph(
        &mut self,
        render_graph: &RenderGraph,
        resources: &mut RenderGraphResources,
    ) -> ash::prelude::VkResult<()> {
        const TIMEOUT_NS: u64 = std::time::Duration::from_secs(2).as_nanos() as u64;

        unsafe {
            self.device
                .core
                .wait_for_fences(&[self.frame_done_fence], true, TIMEOUT_NS)
                .unwrap();

            self.device
                .core
                .reset_fences(&[self.frame_done_fence])
                .unwrap();
        }

        let semaphore_create_info = vk::SemaphoreCreateInfo::builder().build();

        //If we need more semaphores create them
        if self.swapchain_semaphores.len() < render_graph.swapchain_images.len() {
            for _ in self.swapchain_semaphores.len()..render_graph.swapchain_images.len() {
                self.swapchain_semaphores.push(unsafe {
                    (
                        self.device
                            .core
                            .create_semaphore(&semaphore_create_info, None)?,
                        self.device
                            .core
                            .create_semaphore(&semaphore_create_info, None)?,
                    )
                });
            }
        }

        let mut last_usage_swapchain: Vec<ImageAccess> = render_graph
            .swapchain_images
            .iter()
            .map(|_| ImageAccess {
                write: false,
                stage: vk::PipelineStageFlags2::NONE,
                access: vk::AccessFlags2::NONE,
                layout: vk::ImageLayout::UNDEFINED,
            })
            .collect();

        let mut swapchain_image_indices: Vec<(vk::SwapchainKHR, vk::Image, u32)> =
            Vec::with_capacity(render_graph.swapchain_images.len());
        for (surface_handle, swapchain_semaphores) in render_graph
            .swapchain_images
            .iter()
            .zip(self.swapchain_semaphores.iter())
        {
            let swapchain = resources
                .swapchain
                .swapchains
                .get_mut(surface_handle)
                .expect("Failed to find swapchain");

            let mut swapchain_result: ash::prelude::VkResult<(u32, bool)> =
                swapchain.acquire_next_image(swapchain_semaphores.0);
            while swapchain_result == Err(vk::Result::ERROR_OUT_OF_DATE_KHR) {
                info!("Swapchain Out of Data, Rebuilding");
                swapchain.rebuild()?;
                swapchain_result = swapchain.acquire_next_image(swapchain_semaphores.0);
            }

            let index = swapchain_result.unwrap().0;

            swapchain_image_indices.push((
                swapchain.get_handle(),
                swapchain.get_image(index),
                index,
            ));
        }

        unsafe {
            self.device
                .core
                .begin_command_buffer(self.command_buffer, &vk::CommandBufferBeginInfo::builder())
                .unwrap();

            record_single_queue_render_graph_bad_sync(
                render_graph,
                &self.device,
                self.command_buffer,
                resources,
            );

            // Transition Swapchain
            {
                let next_swapchain_access = ImageAccess {
                    write: false,
                    stage: vk::PipelineStageFlags2::ALL_COMMANDS,
                    access: vk::AccessFlags2::MEMORY_READ,
                    layout: vk::ImageLayout::PRESENT_SRC_KHR,
                };

                let swapchain_subresource_range = vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_array_layer(0)
                    .layer_count(1)
                    .base_mip_level(0)
                    .level_count(1)
                    .build();

                let mut image_barriers = Vec::with_capacity(render_graph.swapchain_images.len());

                for i in 0..render_graph.swapchain_images.len() {
                    let last_access = last_usage_swapchain[i].clone();
                    image_barriers.push(
                        vk::ImageMemoryBarrier2::builder()
                            .image(swapchain_image_indices[i].1)
                            .src_stage_mask(last_access.stage)
                            .src_access_mask(last_access.access)
                            .old_layout(last_access.layout)
                            .dst_stage_mask(next_swapchain_access.stage)
                            .dst_access_mask(next_swapchain_access.access)
                            .new_layout(next_swapchain_access.layout)
                            .subresource_range(swapchain_subresource_range)
                            .build(),
                    );
                }

                self.device.core.cmd_pipeline_barrier2(
                    self.command_buffer,
                    &vk::DependencyInfo::builder()
                        .image_memory_barriers(&image_barriers)
                        .build(),
                );
            }

            self.device
                .core
                .end_command_buffer(self.command_buffer)
                .unwrap();

            let command_buffer_info = &[vk::CommandBufferSubmitInfo::builder()
                .command_buffer(self.command_buffer)
                .build()];
            let wait_semaphore_infos: Vec<vk::SemaphoreSubmitInfo> = self
                .swapchain_semaphores
                .iter()
                .map(|(semaphore, _)| {
                    vk::SemaphoreSubmitInfo::builder()
                        .semaphore(*semaphore)
                        .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                        .build()
                })
                .collect();

            let signal_semaphore_infos: Vec<vk::SemaphoreSubmitInfo> = self
                .swapchain_semaphores
                .iter()
                .map(|(_, semaphore)| {
                    vk::SemaphoreSubmitInfo::builder()
                        .semaphore(*semaphore)
                        .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                        .build()
                })
                .collect();

            let submit_info = vk::SubmitInfo2::builder()
                .command_buffer_infos(command_buffer_info)
                .wait_semaphore_infos(&wait_semaphore_infos)
                .signal_semaphore_infos(&signal_semaphore_infos);

            self.device
                .core
                .queue_submit2(
                    self.queue.handle,
                    &[submit_info.build()],
                    self.frame_done_fence,
                )
                .unwrap();

            let mut swapchains = Vec::with_capacity(swapchain_image_indices.len());
            let mut swapchain_indies = Vec::with_capacity(swapchain_image_indices.len());
            let mut wait_semaphores = Vec::with_capacity(swapchain_image_indices.len());

            for ((swapchain_handle, _, swapchain_index), swapchain_semaphores) in
                swapchain_image_indices
                    .iter()
                    .zip(self.swapchain_semaphores.iter())
            {
                swapchains.push(*swapchain_handle);
                swapchain_indies.push(*swapchain_index);
                wait_semaphores.push(swapchain_semaphores.1);
            }

            let _ = self.device.swapchain.queue_present(
                self.queue.handle,
                &vk::PresentInfoKHR::builder()
                    .swapchains(&swapchains)
                    .image_indices(&swapchain_indies)
                    .wait_semaphores(&wait_semaphores),
            );
        }

        Ok(())
    }
}

impl Drop for BasicRenderGraphExecutor {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.core.device_wait_idle();
            self.device
                .core
                .destroy_command_pool(self.command_pool, None);

            for semaphore in self.swapchain_semaphores.drain(..) {
                self.device.core.destroy_semaphore(semaphore.0, None);
                self.device.core.destroy_semaphore(semaphore.1, None);
            }

            self.device.core.destroy_fence(self.frame_done_fence, None);
        }
    }
}

fn record_single_queue_render_graph_bad_sync(
    render_graph: &RenderGraph,
    device: &AshDevice,
    command_buffer: vk::CommandBuffer,
    resources: &mut RenderGraphResources,
) {
    for pass in render_graph.passes.iter() {
        //Bad Barrier
        unsafe {
            device.core.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfo::builder().memory_barriers(&[vk::MemoryBarrier2::builder()
                    .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
                    .build()]),
            );

            if let Some(build_cmd_fn) = &pass.build_cmd_fn {
                build_cmd_fn(device, command_buffer, resources);
            }
        }
    }
}
