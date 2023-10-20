use crate::buffer::{AshBuffer, Buffer};
use crate::descriptor_set::GpuBindingIndex;
use crate::device::{AshDevice, AshQueue};
use crate::image::{vk_format_get_aspect_flags, AshImage, Image};
use crate::render_graph_builder::{
    IndexType, RasterDispatch, RenderGraphBuilder, RenderPass, RenderPassType, ShaderResourceUsage,
    Transfer,
};
use crate::resource_managers::{ResourceManager, TransientResourceManager};
use crate::swapchain::{AcquiredSwapchainImage, SwapchainManager};
use crate::{
    BufferHandle, ImageHandle, RasterPipelineHandle, RasterPipleineKey, Sampler, SamplerHandle,
};
use ash::vk;
use log::info;
use std::sync::Arc;

// Render Graph Executor Evolution
// 0. Whole pipeline barriers between passes, no image layout changes (only general layout), no pass order changes, no dead-code culling
// 1. Specific pipeline barriers between passes with image layout changes, no pass order changes, no dead-code culling
// 2. Whole graph evaluation with pass reordering and dead code culling
// 3. Multi-Queue execution
// 4. Sub resource tracking. Allowing image levels/layers and buffer regions to be transition and accessed in parallel

pub struct BasicRenderGraphExecutor {
    device: Arc<AshDevice>,
    queue: AshQueue,

    pipeline_layout: vk::PipelineLayout,

    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,

    swapchain_semaphores: Vec<(vk::Semaphore, vk::Semaphore)>,
    frame_done_fence: vk::Fence,
}

impl BasicRenderGraphExecutor {
    pub fn new(
        device: Arc<AshDevice>,
        pipeline_layout: vk::PipelineLayout,
        device_queue_index: u32,
    ) -> ash::prelude::VkResult<Self> {
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
            pipeline_layout,
            command_pool,
            command_buffer,
            swapchain_semaphores,
            frame_done_fence,
        })
    }

    pub fn execute_graph(
        &mut self,
        device_transfers: Option<&[Transfer]>,
        render_graph_builder: &RenderGraphBuilder,
        persistent_resource_manager: &mut ResourceManager,
        transient_resource_manager: &mut TransientResourceManager,
        swapchain_manager: &mut SwapchainManager,
        raster_pipelines: &slotmap::SlotMap<RasterPipleineKey, vk::Pipeline>,
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
        if self.swapchain_semaphores.len() < render_graph_builder.swapchain_images.len() {
            for _ in self.swapchain_semaphores.len()..render_graph_builder.swapchain_images.len() {
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

        let mut swapchain_index_images: Vec<AcquiredSwapchainImage> =
            Vec::with_capacity(render_graph_builder.swapchain_images.len());
        for (surface_handle, swapchain_semaphores) in render_graph_builder
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

            let mut swapchain_result: ash::prelude::VkResult<(AcquiredSwapchainImage, bool)> =
                swapchain.acquire_next_image(swapchain_semaphores.0);

            while let Err(vk::Result::ERROR_OUT_OF_DATE_KHR) = &swapchain_result {
                info!("Swapchain Out of Data, Rebuilding");
                swapchain.rebuild()?;
                swapchain_result = swapchain.acquire_next_image(swapchain_semaphores.0);
            }

            let image = swapchain_result.unwrap().0;
            swapchain_index_images.push(image);
        }

        unsafe {
            self.device
                .core
                .begin_command_buffer(self.command_buffer, &vk::CommandBufferBeginInfo::builder())
                .unwrap();

            let pipeline_bind_points = [
                vk::PipelineBindPoint::COMPUTE,
                vk::PipelineBindPoint::GRAPHICS,
            ];
            let layout = self.pipeline_layout;
            let set = persistent_resource_manager.descriptor_set.get_set();
            for pipeline_bind_point in pipeline_bind_points {
                self.device.core.cmd_bind_descriptor_sets(
                    self.command_buffer,
                    pipeline_bind_point,
                    layout,
                    0,
                    &[set],
                    &[],
                );
            }

            // Transition Swapchain to General
            if !swapchain_index_images.is_empty() {
                let swapchain_subresource_range = vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_array_layer(0)
                    .layer_count(1)
                    .base_mip_level(0)
                    .level_count(1)
                    .build();

                let image_barriers: Vec<vk::ImageMemoryBarrier2> = swapchain_index_images
                    .iter()
                    .map(|swapchain_image| {
                        vk::ImageMemoryBarrier2::builder()
                            .image(swapchain_image.image.handle)
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

            transient_resource_manager.resolve_images(
                persistent_resource_manager,
                &swapchain_index_images,
                &render_graph_builder.transient_images,
            );
            transient_resource_manager.resolve_buffers(&render_graph_builder.transient_buffers);
            let resources = RenderGraphResources {
                persistent: persistent_resource_manager,
                swapchain_images: &swapchain_index_images,
                transient_images: &transient_resource_manager.transient_images,
                transient_buffers: &transient_resource_manager.transient_buffers,
                pipeline_layout: self.pipeline_layout,
                raster_pipelines,
            };

            if let Some(transfers) = device_transfers {
                if let Some(debug_util) = &self.device.instance.debug_utils {
                    debug_util.cmd_begin_label(
                        self.command_buffer,
                        "Device Upload Pass",
                        [0.5, 0.0, 0.5, 1.0],
                    );
                }

                self.device.core.cmd_pipeline_barrier2(
                    self.command_buffer,
                    &vk::DependencyInfo::builder().memory_barriers(&[vk::MemoryBarrier2::builder(
                    )
                    .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
                    .build()]),
                );

                record_transfer_pass(&self.device, self.command_buffer, &resources, transfers);

                if let Some(debug_util) = &self.device.instance.debug_utils {
                    debug_util.cmd_end_label(self.command_buffer);
                }
            }

            for render_pass in render_graph_builder.passes.iter() {
                record_render_pass(&self.device, self.command_buffer, &resources, render_pass);
            }

            // Transition Swapchain to Present
            if !swapchain_index_images.is_empty() {
                let swapchain_subresource_range = vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_array_layer(0)
                    .layer_count(1)
                    .base_mip_level(0)
                    .level_count(1)
                    .build();

                let image_barriers: Vec<vk::ImageMemoryBarrier2> = swapchain_index_images
                    .iter()
                    .map(|swapchain_image| {
                        vk::ImageMemoryBarrier2::builder()
                            .image(swapchain_image.image.handle)
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
                [0..swapchain_index_images.len()]
                .iter()
                .map(|(semaphore, _)| {
                    vk::SemaphoreSubmitInfo::builder()
                        .semaphore(*semaphore)
                        .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                        .build()
                })
                .collect();

            let signal_semaphore_infos: Vec<vk::SemaphoreSubmitInfo> = self.swapchain_semaphores
                [0..swapchain_index_images.len()]
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

            let mut swapchains = Vec::with_capacity(swapchain_index_images.len());
            let mut swapchain_indies = Vec::with_capacity(swapchain_index_images.len());
            let mut wait_semaphores = Vec::with_capacity(swapchain_index_images.len());

            for (swapchain_image, swapchain_semaphores) in swapchain_index_images
                .iter()
                .zip(self.swapchain_semaphores.iter())
            {
                swapchains.push(swapchain_image.swapchain_handle);
                swapchain_indies.push(swapchain_image.image_index);
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

fn record_render_pass(
    device: &AshDevice,
    command_buffer: vk::CommandBuffer,
    resources: &RenderGraphResources,
    render_pass: &RenderPass,
) {
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
    }

    //TODO: use queue
    let _ = render_pass.queue;

    if let Some(debug_util) = &device.instance.debug_utils {
        debug_util.cmd_begin_label(
            command_buffer,
            &render_pass.label_name,
            render_pass.label_color,
        );
    }

    match &render_pass.pass_type {
        RenderPassType::Transfer { transfers } => {
            record_transfer_pass(device, command_buffer, resources, transfers);
        }
        RenderPassType::Compute { .. } => {
            todo!("Compute pass")
        }
        RenderPassType::Raster {
            framebuffer,
            draw_commands,
        } => {
            record_raster_pass(
                device,
                command_buffer,
                resources,
                framebuffer,
                draw_commands,
            );
        }
    }

    if let Some(debug_util) = &device.instance.debug_utils {
        debug_util.cmd_end_label(command_buffer);
    }
}

fn record_transfer_pass(
    device: &AshDevice,
    command_buffer: vk::CommandBuffer,
    resources: &RenderGraphResources,
    transfers: &[Transfer],
) {
    for transfer in transfers.iter() {
        match transfer {
            Transfer::CopyBufferToBuffer {
                src,
                dst,
                copy_size,
            } => {
                let src_buffer = resources.get_buffer(src.buffer);
                let dst_buffer = resources.get_buffer(dst.buffer);
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
            Transfer::CopyBufferToImage {
                src,
                dst,
                copy_size,
            } => {
                let src_buffer = resources.get_buffer(src.buffer);
                let dst_image = resources.get_image(dst.image);
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
            Transfer::CopyImageToBuffer {
                src,
                dst,
                copy_size,
            } => {
                let src_image = resources.get_image(src.image);
                let dst_buffer = resources.get_buffer(dst.buffer);
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
            Transfer::CopyImageToImage {
                src,
                dst,
                copy_size,
            } => {
                let src_image = resources.get_image(src.image);
                let dst_image = resources.get_image(dst.image);

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

fn record_raster_pass(
    device: &AshDevice,
    command_buffer: vk::CommandBuffer,
    resources: &RenderGraphResources,
    framebuffer: &crate::render_graph_builder::Framebuffer,
    draw_commands: &[crate::render_graph_builder::RasterDrawCommand],
) {
    //Begin Rendering
    {
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

        let depth_stencil_attachment_info: vk::RenderingAttachmentInfo;
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
                resources.get_raster_pipeline(draw_call.pipeline),
            );
        }

        //Bind Vertex Buffers
        if !draw_call.vertex_buffers.is_empty() {
            let mut vertex_buffers: Vec<vk::Buffer> =
                Vec::with_capacity(draw_call.vertex_buffers.len());
            for vertex_buffer in draw_call.vertex_buffers.iter() {
                vertex_buffers.push(resources.get_buffer(vertex_buffer.buffer).handle);
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

        //Bind Index Buffer
        if let Some((index_buffer, index_type)) = draw_call.index_buffer {
            unsafe {
                device.core.cmd_bind_index_buffer(
                    command_buffer,
                    resources.get_buffer(index_buffer.buffer).handle,
                    index_buffer.offset as vk::DeviceSize,
                    match index_type {
                        IndexType::U16 => vk::IndexType::UINT16,
                        IndexType::U32 => vk::IndexType::UINT32,
                    },
                );
            }
        }

        //Push Resource
        unsafe {
            let mut push_bindings: Vec<GpuBindingIndex> = Vec::new();

            for resource in draw_call.resources.iter() {
                push_bindings.push(match resource {
                    ShaderResourceUsage::StorageBuffer { buffer, .. } => resources
                        .get_buffer(*buffer)
                        .storage_binding
                        .expect("Buffer not bound as storage buffer"),
                    ShaderResourceUsage::StorageImage { image, .. } => resources
                        .get_image(*image)
                        .storage_binding
                        .expect("Image not bound as storage image"),
                    ShaderResourceUsage::SampledImage(handle) => resources
                        .get_image(*handle)
                        .sampled_binding
                        .expect("Image not bound as sampled image"),
                    ShaderResourceUsage::Sampler(handle) => resources
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

            device.core.cmd_push_constants(
                command_buffer,
                resources.get_pipeline_layout(),
                vk::ShaderStageFlags::ALL,
                0,
                &push_data_bytes,
            );
        }

        //Dispatch
        unsafe {
            match &draw_call.dispatch {
                RasterDispatch::Draw {
                    vertices,
                    instances,
                } => device.core.cmd_draw(
                    command_buffer,
                    vertices.len() as u32,
                    instances.len() as u32,
                    vertices.start,
                    instances.start,
                ),
                RasterDispatch::DrawIndexed {
                    base_vertex,
                    indices,
                    instances,
                } => device.core.cmd_draw_indexed(
                    command_buffer,
                    indices.len() as u32,
                    instances.len() as u32,
                    indices.start,
                    *base_vertex,
                    instances.start,
                ),
                RasterDispatch::DrawIndirect {
                    buffer,
                    draw_count,
                    stride,
                } => device.core.cmd_draw_indirect(
                    command_buffer,
                    resources.get_buffer(buffer.buffer).handle,
                    buffer.offset as vk::DeviceSize,
                    *draw_count,
                    *stride,
                ),
                RasterDispatch::DrawIndirectIndexed {
                    buffer,
                    draw_count,
                    stride,
                } => device.core.cmd_draw_indexed_indirect(
                    command_buffer,
                    resources.get_buffer(buffer.buffer).handle,
                    buffer.offset as vk::DeviceSize,
                    *draw_count,
                    *stride,
                ),
            }
        }
    }

    //End Rendering
    unsafe {
        device.core.cmd_end_rendering(command_buffer);
    }
}

// Render Graph Executor Evolution
// 0. Whole pipeline barriers between passes, no image layout changes (only general layout), no pass order changes, no dead-code culling
// 1. Specific pipeline barriers between passes with image layout changes, no pass order changes, no dead-code culling
// 2. Whole graph evaluation with pass reordering and dead code culling
// 3. Multi-Queue execution
// 4. Sub resource tracking. Allowing image levels/layers and buffer regions to be transition and accessed in parallel

pub struct RenderGraphResources<'a> {
    pub(crate) persistent: &'a mut ResourceManager,
    pub(crate) swapchain_images: &'a [AcquiredSwapchainImage],
    pub(crate) transient_images: &'a [Image],
    pub(crate) transient_buffers: &'a [Buffer],

    pub(crate) pipeline_layout: vk::PipelineLayout,
    pub(crate) raster_pipelines: &'a slotmap::SlotMap<RasterPipleineKey, vk::Pipeline>,
}

impl<'a> RenderGraphResources<'a> {
    pub fn get_buffer(&self, resource: BufferHandle) -> AshBuffer {
        match resource {
            BufferHandle::Persistent(buffer_key) => self
                .persistent
                .get_buffer(buffer_key)
                .expect("render pass tried to access invalid persistent buffer")
                .get_copy(),
            BufferHandle::Transient(index) => self.transient_buffers[index].get_copy(),
        }
    }

    pub fn get_image(&self, resource: ImageHandle) -> AshImage {
        match resource {
            ImageHandle::Persistent(image_key) => self
                .persistent
                .get_image(image_key)
                .expect("Invalid Image Key")
                .get_copy(),
            ImageHandle::Transient(index) => self.transient_images[index].get_copy(),
            ImageHandle::Swapchain(index) => self.swapchain_images[index].image,
        }
    }

    pub(crate) fn get_sampler(&self, resource: SamplerHandle) -> Arc<Sampler> {
        self.persistent
            .get_sampler(resource.0)
            .expect("Invalid Sampler Key")
    }

    pub fn get_raster_pipeline(&self, pipeline: RasterPipelineHandle) -> vk::Pipeline {
        *self.raster_pipelines.get(pipeline.0).unwrap()
    }

    pub fn get_pipeline_layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }
}
