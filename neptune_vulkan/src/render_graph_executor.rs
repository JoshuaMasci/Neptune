use crate::device::{AshDevice, AshQueue};
use crate::render_graph_builder::{
    IndexType, RasterDispatch, RenderGraphBuilder, RenderPassType, ShaderResourceUsage,
};
use crate::resource_managers::{ResourceManager, TransientResourceManager};
use crate::swapchain::{SwapchainImage, SwapchainManager};
use crate::RasterPipleineKey;
use ash::vk;
use log::info;
use std::sync::Arc;

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
        upload_data: Option<()>,
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

        let mut swapchain_image: Vec<(vk::SwapchainKHR, SwapchainImage)> =
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
                &render_graph_builder.transient_images,
            );
            let transient_buffers =
                transient_resource_manager.resolve_buffers(&render_graph_builder.transient_buffers);
            let resources = crate::RenderGraphResources {
                persistent: persistent_resource_manager,
                swapchain_images: &swapchain_image,
                transient_images: &transient_images,
                transient_buffers,
                pipeline_layout: self.pipeline_layout,
                raster_pipelines,
            };

            for render_pass in render_graph_builder.passes.iter() {
                //TODO: use queue
                let _ = render_pass.queue;

                if let Some(debug_util) = &self.device.instance.debug_utils {
                    debug_util.cmd_begin_label(
                        self.command_buffer,
                        &render_pass.lable_name,
                        render_pass.lable_color,
                    );
                }

                match &render_pass.pass_type {
                    RenderPassType::Compute { .. } => {
                        todo!("Compute pass")
                    }
                    RenderPassType::Raster {
                        framebuffer,
                        draw_commands,
                    } => {
                        record_raster_pass(
                            &self.device,
                            self.command_buffer,
                            framebuffer,
                            draw_commands,
                            &resources,
                        );
                    }
                }

                if let Some(debug_util) = &self.device.instance.debug_utils {
                    debug_util.cmd_end_label(self.command_buffer);
                }
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

fn record_raster_pass(
    device: &AshDevice,
    command_buffer: vk::CommandBuffer,
    framebuffer: &crate::render_graph_builder::Framebuffer,
    draw_commands: &[crate::render_graph_builder::RasterDrawCommand],
    resources: &crate::RenderGraphResources,
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
            let mut push_data: Vec<u32> = Vec::new();

            //TODO: get binding
            for resource in draw_call.resources.iter() {
                push_data.push(match resource {
                    ShaderResourceUsage::StorageBuffer { .. } => 0,
                    ShaderResourceUsage::StorageImage { .. } => 0,
                    ShaderResourceUsage::SampledImage(_handle) => 0,
                    ShaderResourceUsage::Sampler(_handle) => 0,
                });
            }

            let push_data_bytes: &[u8] = std::slice::from_raw_parts(
                push_data.as_ptr() as *const u8,
                std::mem::size_of_val(&push_data),
            );

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
