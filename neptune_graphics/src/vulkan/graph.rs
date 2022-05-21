use crate::render_graph::{
    BufferAccess, BufferId, RenderPassData, ResourceAccess, ResourceAccessType, TextureAccess,
    TextureId,
};
use crate::vulkan::pipeline_cache::{FramebufferLayout, PipelineCache};
use ash::vk;
use std::collections::HashMap;
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
pub(crate) struct PassSetBarrier {
    pub(crate) memory_barriers: Vec<vk::MemoryBarrier2>,
    pub(crate) buffer_barriers: Vec<vk::BufferMemoryBarrier2>,
    pub(crate) image_barriers: Vec<vk::ImageMemoryBarrier2>,
}

impl PassSetBarrier {
    pub(crate) fn record(&self, device: &Rc<ash::Device>, command_buffer: vk::CommandBuffer) {
        unsafe {
            device.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfo::builder()
                    .memory_barriers(&self.memory_barriers)
                    .buffer_memory_barriers(&self.buffer_barriers)
                    .image_memory_barriers(&self.image_barriers)
                    .build(),
            );
        }
    }
}

#[derive(Default)]
pub(crate) struct PassSet {
    pub(crate) pre_barrier: PassSetBarrier,
    pub(crate) passes: Vec<Pass>,
    pub(crate) post_barrier: PassSetBarrier,
}

#[derive(Default)]
pub struct Graph {
    pub sets: Vec<PassSet>,
}

enum RenderGraphEdge {
    Buffer {
        id: BufferId,
        last: BufferAccess,
        next: BufferAccess,
    },
    Texture {
        id: TextureId,
        last: TextureAccess,
        next: TextureAccess,
    },
}

fn get_buffer_node_access_list(
    access_type: &ResourceAccessType<BufferAccess>,
) -> Vec<ResourceAccess<BufferAccess>> {
    match access_type {
        ResourceAccessType::Write(access) => vec![*access],
        ResourceAccessType::Reads(some) => some.clone(),
    }
}

impl Graph {
    pub fn new(mut render_graph: crate::render_graph::RenderGraphBuilder) -> Self {
        //Construct the graph
        let mut graph: crate::render_graph::graph::Graph<
            crate::render_graph::RenderPass,
            RenderGraphEdge,
        > = crate::render_graph::graph::Graph::new();

        //Add Nodes
        let pass_to_node_id: Vec<crate::render_graph::graph::NodeId> = render_graph
            .passes
            .drain(..)
            .map(|pass| graph.add_node(pass))
            .collect();

        //Add Buffer Edges
        for buffer in render_graph.buffers.iter() {
            let mut access_list = buffer.access_list.iter();
            if let Some(mut last_access) = access_list.next() {
                for access in access_list {
                    let last = get_buffer_node_access_list(last_access);
                    let current = get_buffer_node_access_list(last_access);

                    for current_access in current {
                        for last_access in last.iter() {
                            graph.add_edge(
                                pass_to_node_id[last_access.pass_id],
                                pass_to_node_id[current_access.pass_id],
                                RenderGraphEdge::Buffer {
                                    id: buffer.id,
                                    last: last_access.access,
                                    next: current_access.access,
                                },
                            );
                        }
                    }

                    last_access = access;
                }
            }
        }

        //Add Texture Edges

        let mut sets = Vec::new();

        while let Some(mut nodes) = graph.get_unconnected_nodes() {
            let mut pass_set = PassSet::default();

            for node in nodes.drain(..) {
                for edge in node.edges {
                    match edge.data {
                        RenderGraphEdge::Buffer { id, last, next } => {
                            let handle = vk::Buffer::null(); //TODO: this
                            let src = last.get_vk();
                            let dst = next.get_vk();

                            pass_set.buffer_barriers.push(
                                vk::BufferMemoryBarrier2::builder()
                                    .buffer(handle)
                                    .offset(0)
                                    .size(vk::WHOLE_SIZE)
                                    .src_stage_mask(src.0)
                                    .src_access_mask(src.1)
                                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                                    .dst_stage_mask(dst.0)
                                    .dst_access_mask(dst.1)
                                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                                    .build(),
                            );
                        }
                        RenderGraphEdge::Texture { .. } => {}
                    }
                }

                let data = match node.data.data {
                    RenderPassData::Raster {
                        color_attachments,
                        depth_stencil_attachment,
                        raster_fn,
                    } => PassData::Raster {
                        framebuffer_layout: FramebufferLayout {
                            color_attachments: vec![],
                            depth_stencil_attachment: None,
                        },
                        render_area: Default::default(),
                        color_attachments: vec![],
                        depth_stencil_attachment: None,
                        raster_fn: None,
                    },
                };

                pass_set.passes.push(Pass {
                    id: node.data.id,
                    name: node.data.name,
                    data,
                });
            }

            sets.push(pass_set);
        }

        Self { sets }
    }

    pub fn record_command_buffer(
        &mut self,
        device: &Rc<ash::Device>,
        command_buffer: vk::CommandBuffer,
        pipeline_cache: &mut PipelineCache,
    ) {
        for set in self.sets.iter_mut() {
            set.pre_barrier.record(device, command_buffer);

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

            set.pre_barrier.record(device, command_buffer);
        }
    }
}
