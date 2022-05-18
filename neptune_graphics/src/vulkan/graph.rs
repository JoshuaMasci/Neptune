use crate::vulkan::pipeline_cache::{FramebufferLayout, PipelineCache};
use ash::vk;
use std::rc::Rc;

type RasterFnVulkan =
    dyn FnOnce(&Rc<ash::Device>, vk::CommandBuffer, &mut PipelineCache, &FramebufferLayout);

pub enum PassData {
    None,
    Raster {
        framebuffer_layout: FramebufferLayout,
        render_area: vk::Rect2D,
        color_attachments: Vec<vk::RenderingAttachmentInfoKHR>,
        depth_stencil_attachment: Option<vk::RenderingAttachmentInfoKHR>,
        raster_fn: Option<Box<RasterFnVulkan>>,
    },
    Compute {
        pipeline: vk::Pipeline,
        dispatch_size: [u32; 3],
    },
    // Raytrace,
    // Custom,
}

pub struct Pass {
    pub id: usize,
    pub name: String,
    pub data: PassData,
}

#[derive(Default)]
pub struct PassSet {
    pub memory_barriers: Vec<vk::MemoryBarrier2>,
    pub buffer_barriers: Vec<vk::BufferMemoryBarrier2>,
    pub image_barriers: Vec<vk::ImageMemoryBarrier2>,
    pub passes: Vec<Pass>,
}

#[derive(Default)]
pub struct Graph {
    pub sets: Vec<PassSet>,
}

impl Graph {
    pub fn new(render_graph: crate::render_graph::RenderGraphBuilder) -> Self {
        
        
        
        Self { sets: vec![] }
    }

    pub fn record_command_buffer(
        &mut self,
        device: &Rc<ash::Device>,
        command_buffer: vk::CommandBuffer,
        pipeline_cache: &mut PipelineCache,
    ) {
        for set in self.sets.iter_mut() {
            unsafe {
                device.cmd_pipeline_barrier2(
                    command_buffer,
                    &vk::DependencyInfo::builder()
                        .memory_barriers(&set.memory_barriers)
                        .buffer_memory_barriers(&set.buffer_barriers)
                        .image_memory_barriers(&set.image_barriers)
                        .build(),
                );
            }

            for pass in set.passes.iter_mut() {
                println!("Render Pass {}: {}", pass.id, pass.name);
                //TODO: set push constants
                match &mut pass.data {
                    PassData::None => {}
                    PassData::Raster {
                        framebuffer_layout,
                        render_area,
                        color_attachments,
                        depth_stencil_attachment,
                        raster_fn,
                    } => {
                        let mut rendering_info = vk::RenderingInfoKHR::builder()
                            .render_area(*render_area)
                            .layer_count(1)
                            .color_attachments(color_attachments);
                        if let Some(depth_stencil_attachment) = depth_stencil_attachment {
                            rendering_info =
                                rendering_info.depth_attachment(depth_stencil_attachment);
                            rendering_info =
                                rendering_info.stencil_attachment(depth_stencil_attachment);
                        }

                        unsafe {
                            device.cmd_begin_rendering(command_buffer, &rendering_info);
                        }

                        if let Some(raster_fn) = raster_fn.take() {
                            raster_fn(device, command_buffer, pipeline_cache, framebuffer_layout);
                        }

                        unsafe {
                            device.cmd_end_rendering(command_buffer);
                        }
                    }
                    PassData::Compute {
                        pipeline,
                        dispatch_size,
                    } => unsafe {
                        device.cmd_bind_pipeline(
                            command_buffer,
                            vk::PipelineBindPoint::COMPUTE,
                            *pipeline,
                        );
                        device.cmd_dispatch(
                            command_buffer,
                            dispatch_size[0],
                            dispatch_size[1],
                            dispatch_size[2],
                        );
                    },
                }
            }
        }
    }
}
