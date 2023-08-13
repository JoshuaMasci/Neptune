use crate::{BufferHandle, BufferKey, ImageHandle, ImageKey, SurfaceHandle};
use ash::vk;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct BufferAccess {
    pub write: bool, //TODO: calculate this from stage+access?
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
}

#[derive(Clone, Debug)]
pub struct ImageAccess {
    pub write: bool, //TODO: calculate this from stage+access+layout?
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
    pub layout: vk::ImageLayout,
}

pub type BuildCommandFn = dyn Fn(&AshDevice, vk::CommandBuffer, &mut RenderGraphResources);

pub struct ColorAttachment {
    pub image: ImageHandle,
    pub clear: Option<[f32; 4]>,
}

impl ColorAttachment {
    pub fn new(image: ImageHandle) -> Self {
        Self { image, clear: None }
    }

    pub fn new_clear(image: ImageHandle, clear: [f32; 4]) -> Self {
        Self {
            image,
            clear: Some(clear),
        }
    }
}

pub struct DepthStencilAttachment {
    pub image: ImageHandle,
    pub clear: Option<(f32, u32)>,
}

impl DepthStencilAttachment {
    pub fn new(image: ImageHandle) -> Self {
        Self { image, clear: None }
    }

    pub fn new_clear(image: ImageHandle, clear: (f32, u32)) -> Self {
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
    pub input_attachments: Vec<ImageHandle>,
}

#[derive(Default)]
pub struct RenderPass {
    pub name: String,
    pub queue: vk::Queue,
    pub buffer_usages: HashMap<BufferHandle, BufferAccess>,
    pub image_usages: HashMap<ImageHandle, ImageAccess>,
    pub framebuffer: Option<Framebuffer>,
    pub build_cmd_fn: Option<Box<BuildCommandFn>>,
}

#[derive(Debug, Clone)]
pub enum TransientImageSize {
    Exact(vk::Extent2D),
    Relative([f32; 2], ImageHandle),
}

#[derive(Debug, Clone)]
pub struct TransientImageDesc {
    pub size: TransientImageSize,
    pub format: vk::Format,
    pub usage: vk::ImageUsageFlags,
    pub mip_levels: u32,
    pub memory_location: gpu_allocator::MemoryLocation,
}

#[derive(Default)]
pub struct RenderGraph {
    pub(crate) transient_buffers: Vec<BufferDesc>,
    pub(crate) transient_images: Vec<TransientImageDesc>,
    pub(crate) swapchain_images: Vec<SurfaceHandle>,
    pub(crate) passes: Vec<RenderPass>,
}

impl RenderGraph {
    pub fn create_transient_buffer(&mut self, desc: BufferDesc) -> BufferHandle {
        let index = self.transient_buffers.len();
        self.transient_buffers.push(desc);
        BufferHandle::Transient(index)
    }

    pub fn create_transient_image(&mut self, desc: TransientImageDesc) -> ImageHandle {
        let index = self.transient_images.len();
        self.transient_images.push(desc);
        ImageHandle::Transient(index)
    }

    pub fn acquire_swapchain_image(&mut self, surface_handle: SurfaceHandle) -> ImageHandle {
        let index = self.swapchain_images.len();
        self.swapchain_images.push(surface_handle);
        ImageHandle::Swapchain(index)
    }

    pub fn add_pass(&mut self, pass: RenderPass) {
        self.passes.push(pass);
    }
}

#[derive(Copy, Clone)]
pub struct VkImage {
    pub handle: vk::Image,
    pub view: vk::ImageView,
    pub size: vk::Extent2D,
    pub format: vk::Format,
}

pub struct RenderGraphResources<'a> {
    persistent: &'a mut PersistentResourceManager,
    swapchain_images: &'a [(vk::SwapchainKHR, SwapchainImage)],
    transient_images: &'a [VkImage],
    transient_buffers: &'a [Buffer],
}

impl<'a> RenderGraphResources<'a> {
    pub fn get_image(&self, resource: ImageHandle) -> VkImage {
        match resource {
            ImageHandle::Persistent(image_key) => {
                let image = self
                    .persistent
                    .get_image(image_key)
                    .expect("Invalid Image Key");
                VkImage {
                    handle: image.handle,
                    view: image.view,
                    size: image.extend,
                    format: image.format,
                }
            }
            ImageHandle::Transient(index) => self.transient_images[index],
            ImageHandle::Swapchain(index) => {
                let swapchain_image = self.swapchain_images[index].clone();
                VkImage {
                    handle: swapchain_image.1.handle,
                    view: swapchain_image.1.view,
                    size: swapchain_image.1.extent,
                    format: swapchain_image.1.format,
                }
            }
        }
    }

    pub fn get_buffer(&self, resource: BufferHandle) -> &Buffer {
        match resource {
            BufferHandle::Persistent(buffer_key) => self
                .persistent
                .get_buffer(buffer_key)
                .expect("render pass tried to access invalid persistent buffer"),
            BufferHandle::Transient(index) => &self.transient_buffers[index],
        }
    }
}

use crate::buffer::{Buffer, BufferDesc};
use crate::device::{AshDevice, AshQueue};
use crate::resource_managers::{PersistentResourceManager, TransientResourceManager};
use crate::swapchain::{SwapchainImage, SwapchainManager};
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
    pub fn new(device: Arc<AshDevice>, device_queue_index: u32) -> ash::prelude::VkResult<Self> {
        let queue = device.queues[device_queue_index as usize].clone();

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
        transfer_pass: Option<RenderPass>,
        render_graph: &RenderGraph,
        persistent_resource_manager: &mut PersistentResourceManager,
        transient_resource_manager: &mut TransientResourceManager,
        swapchain_manager: &mut SwapchainManager,
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

        transient_resource_manager.flush();

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

        let mut swapchain_image: Vec<(vk::SwapchainKHR, SwapchainImage)> =
            Vec::with_capacity(render_graph.swapchain_images.len());
        for (surface_handle, swapchain_semaphores) in render_graph
            .swapchain_images
            .iter()
            .map(|surface_handle| {
                self.device
                    .instance
                    .surface_list
                    .get(surface_handle.0)
                    .expect("Failed to find surface")
            })
            .zip(self.swapchain_semaphores.iter())
        {
            let swapchain = swapchain_manager
                .swapchains
                .get_mut(&surface_handle)
                .expect("Failed to find swapchain");

            let mut swapchain_result: ash::prelude::VkResult<(u32, bool)> =
                swapchain.acquire_next_image(swapchain_semaphores.0);
            while swapchain_result == Err(vk::Result::ERROR_OUT_OF_DATE_KHR) {
                info!("Swapchain Out of Data, Rebuilding");
                swapchain.rebuild()?;
                swapchain_result = swapchain.acquire_next_image(swapchain_semaphores.0);
            }

            let index = swapchain_result.unwrap().0;

            swapchain_image.push((swapchain.get_handle(), swapchain.get_image(index)));
        }

        unsafe {
            self.device
                .core
                .begin_command_buffer(self.command_buffer, &vk::CommandBufferBeginInfo::builder())
                .unwrap();

            // Transition Swapchain to General
            if !swapchain_image.is_empty() {
                let swapchain_subresource_range = vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_array_layer(0)
                    .layer_count(1)
                    .base_mip_level(0)
                    .level_count(1)
                    .build();

                let image_barriers: Vec<vk::ImageMemoryBarrier2> = swapchain_image
                    .iter()
                    .map(|(_swapchain, image)| {
                        vk::ImageMemoryBarrier2::builder()
                            .image(image.handle)
                            .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                            .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
                            .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                            .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
                            .old_layout(vk::ImageLayout::UNDEFINED)
                            .new_layout(vk::ImageLayout::GENERAL)
                            .subresource_range(swapchain_subresource_range)
                            .build()
                    })
                    .collect();

                self.device.core.cmd_pipeline_barrier2(
                    self.command_buffer,
                    &vk::DependencyInfo::builder()
                        .image_memory_barriers(&image_barriers)
                        .build(),
                );
            }

            let transient_images = transient_resource_manager.resolve_images(
                persistent_resource_manager,
                &swapchain_image,
                &render_graph.transient_images,
            );

            let transient_buffers =
                transient_resource_manager.resolve_buffers(&render_graph.transient_buffers);

            let mut resources = RenderGraphResources {
                persistent: persistent_resource_manager,
                swapchain_images: &swapchain_image,
                transient_images: &transient_images,
                transient_buffers,
            };

            if let Some(transfer_pass) = transfer_pass {
                record_single_queue_render_pass_bad_sync(
                    &transfer_pass,
                    &self.device,
                    self.command_buffer,
                    &mut resources,
                );
            }

            for render_pass in render_graph.passes.iter() {
                record_single_queue_render_pass_bad_sync(
                    render_pass,
                    &self.device,
                    self.command_buffer,
                    &mut resources,
                );
            }

            // Transition Swapchain to Present
            if !swapchain_image.is_empty() {
                let swapchain_subresource_range = vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_array_layer(0)
                    .layer_count(1)
                    .base_mip_level(0)
                    .level_count(1)
                    .build();

                let image_barriers: Vec<vk::ImageMemoryBarrier2> = swapchain_image
                    .iter()
                    .map(|(_swapchain, image)| {
                        vk::ImageMemoryBarrier2::builder()
                            .image(image.handle)
                            .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                            .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
                            .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                            .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
                            .old_layout(vk::ImageLayout::GENERAL)
                            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                            .subresource_range(swapchain_subresource_range)
                            .build()
                    })
                    .collect();

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
            let wait_semaphore_infos: Vec<vk::SemaphoreSubmitInfo> = self.swapchain_semaphores
                [0..swapchain_image.len()]
                .iter()
                .map(|(semaphore, _)| {
                    vk::SemaphoreSubmitInfo::builder()
                        .semaphore(*semaphore)
                        .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                        .build()
                })
                .collect();

            let signal_semaphore_infos: Vec<vk::SemaphoreSubmitInfo> = self.swapchain_semaphores
                [0..swapchain_image.len()]
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

            let mut swapchains = Vec::with_capacity(swapchain_image.len());
            let mut swapchain_indies = Vec::with_capacity(swapchain_image.len());
            let mut wait_semaphores = Vec::with_capacity(swapchain_image.len());

            for ((swapchain_handle, swapchain_image), swapchain_semaphores) in
                swapchain_image.iter().zip(self.swapchain_semaphores.iter())
            {
                swapchains.push(*swapchain_handle);
                swapchain_indies.push(swapchain_image.index);
                wait_semaphores.push(swapchain_semaphores.1);
            }

            if !swapchains.is_empty() {
                let _ = self.device.swapchain.queue_present(
                    self.queue.handle,
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

fn record_single_queue_render_pass_bad_sync(
    render_pass: &RenderPass,
    device: &AshDevice,
    command_buffer: vk::CommandBuffer,
    resources: &mut RenderGraphResources,
) {
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

        if let Some(debug_util) = &device.instance.debug_utils {
            debug_util.cmd_begin_label(command_buffer, &render_pass.name, [0.0, 1.0, 0.0, 1.0]);
        }

        if let Some(framebuffer) = &render_pass.framebuffer {
            let mut rendering_info_builder = vk::RenderingInfo::builder().layer_count(1);

            let mut extent = None;
            let mut color_attachments = Vec::new();

            for color_attachment in framebuffer.color_attachments.iter() {
                let image = resources.get_image(color_attachment.image);

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

            let mut depth_stencil_attachment_info = vk::RenderingAttachmentInfo::default();

            if let Some(depth_stencil_image) = &framebuffer.depth_stencil_attachment {
                let image = resources.get_image(depth_stencil_image.image);

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

            device
                .core
                .cmd_begin_rendering(command_buffer, &rendering_info_builder);
            _ = depth_stencil_attachment_info;

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

        if let Some(build_cmd_fn) = &render_pass.build_cmd_fn {
            build_cmd_fn(device, command_buffer, resources);
        }

        if render_pass.framebuffer.is_some() {
            device.core.cmd_end_rendering(command_buffer);
        }

        if let Some(debug_util) = &device.instance.debug_utils {
            debug_util.cmd_end_label(command_buffer);
        }
    }
}

// Render Graph Executor Evolution
// 0. Whole pipeline barriers between passes, no image layout changes (only general layout), no pass order changes, no dead-code culling
// 1. Specific pipeline barriers between passes with image layout changes, no pass order changes, no dead-code culling
// 2. Whole graph evaluation with pass reordering and dead code culling
// 3. Multi-Queue execution
// 4. Sub resource tracking. Allowing image levels/layers and buffer regions to be transition and accessed in parallel
