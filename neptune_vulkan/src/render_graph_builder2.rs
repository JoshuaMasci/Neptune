use crate::render_graph::{
    BufferGraphResource, BufferIndex, BufferResourceDescription, ColorAttachment, CommandBuffer,
    CommandBufferDependency, CompiledRenderGraph, DepthStencilAttachment, Framebuffer,
    ImageGraphResource, ImageIndex, ImageResourceDescription, QueueType, RenderPassCommand,
};
use crate::render_graph_builder::{BufferOffset, ImageCopyBuffer, ImageCopyImage};
use crate::resource_managers::{BufferResourceAccess, ImageResourceAccess};
use crate::{
    BufferDescription, BufferHandle, ComputePipelineHandle, ImageHandle, SurfaceHandle,
    TransientImageDesc,
};
use ash::vk;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct RenderGraphBuilder {
    render_graph: CompiledRenderGraph,
    buffer_index_map: HashMap<BufferHandle, BufferIndex>,
    image_index_map: HashMap<ImageHandle, ImageIndex>,
}

impl RenderGraphBuilder {
    pub fn create_transient_buffer(&mut self, desc: BufferDescription) -> BufferHandle {
        let index = self.render_graph.buffer_resources.len() as BufferIndex;
        self.render_graph
            .buffer_resources
            .push(BufferGraphResource {
                description: BufferResourceDescription::Transient(desc),
                last_access: BufferResourceAccess::None,
            });
        let handle = BufferHandle::Transient(index);
        self.buffer_index_map.insert(handle, index);
        handle
    }

    pub fn create_transient_image(&mut self, desc: TransientImageDesc) -> ImageHandle {
        let index = self.render_graph.image_resources.len() as ImageIndex;
        self.render_graph.image_resources.push(ImageGraphResource {
            description: ImageResourceDescription::Transient(desc),
            last_access: ImageResourceAccess::None,
        });
        let handle = ImageHandle::Transient(index);
        self.image_index_map.insert(handle, index);
        handle
    }

    pub fn acquire_swapchain_image(&mut self, surface_handle: SurfaceHandle) -> ImageHandle {
        let index = self.render_graph.image_resources.len() as ImageIndex;
        let swapchain_index = self.render_graph.swapchain_images.len();
        self.render_graph
            .swapchain_images
            .push((surface_handle, index));
        self.render_graph.image_resources.push(ImageGraphResource {
            description: ImageResourceDescription::Swapchain(swapchain_index),
            last_access: ImageResourceAccess::None,
        });
        let handle = ImageHandle::Transient(index);
        self.image_index_map.insert(handle, index);
        handle
    }

    pub fn add_transfer_pass(
        &mut self,
        name: String,
        color: [f32; 4],
        queue: QueueType,
        transfers: &[crate::render_graph_builder::Transfer],
    ) {
        //TODO: queue
        let _ = queue;

        let transfers: Vec<crate::render_graph::Transfer> = transfers
            .iter()
            .map(|transfer| match transfer {
                crate::render_graph_builder::Transfer::CopyBufferToBuffer {
                    src,
                    dst,
                    copy_size,
                } => crate::render_graph::Transfer::BufferToBuffer {
                    src: self.get_buffer_offset(*src),
                    dst: self.get_buffer_offset(*dst),
                    copy_size: *copy_size as vk::DeviceSize,
                },
                crate::render_graph_builder::Transfer::CopyBufferToImage {
                    src,
                    dst,
                    copy_size,
                } => crate::render_graph::Transfer::BufferToImage {
                    src: self.get_image_copy_buffer(*src),
                    dst: self.get_image_copy_image(*dst),
                    copy_size: *copy_size,
                },
                crate::render_graph_builder::Transfer::CopyImageToBuffer {
                    src,
                    dst,
                    copy_size,
                } => crate::render_graph::Transfer::ImageToBuffer {
                    src: self.get_image_copy_image(*src),
                    dst: self.get_image_copy_buffer(*dst),
                    copy_size: *copy_size,
                },
                crate::render_graph_builder::Transfer::CopyImageToImage {
                    src,
                    dst,
                    copy_size,
                } => crate::render_graph::Transfer::ImageToImage {
                    src: self.get_image_copy_image(*src),
                    dst: self.get_image_copy_image(*dst),
                    copy_size: *copy_size,
                },
            })
            .collect();
        let pass = crate::render_graph::RenderPass2 {
            label_name: name,
            label_color: color,
            command: Some(RenderPassCommand::Transfer { transfers }),
        };

        let pass_set = crate::render_graph::RenderPassSet {
            memory_barriers: vec![vk::MemoryBarrier2::builder()
                .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .src_access_mask(vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .dst_access_mask(vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE)
                .build()],
            buffer_barriers: vec![],
            image_barriers: vec![],
            render_passes: vec![pass],
        };

        if self.render_graph.command_buffers.is_empty() {
            self.render_graph
                .command_buffers
                .push(CommandBuffer::default());
        }

        self.render_graph.command_buffers[0]
            .render_pass_sets
            .push(pass_set);
    }

    pub fn add_compute_pass(
        &mut self,
        name: String,
        color: [f32; 4],
        queue: QueueType,
        pipeline: ComputePipelineHandle,
        dispatch_size: [u32; 3],
        resources: &[crate::render_graph_builder::ShaderResourceUsage],
    ) {
    }

    pub fn add_raster_pass(
        &mut self,
        name: String,
        color: [f32; 4],
        color_attachments: &[(ImageHandle, Option<[f32; 4]>)],
        depth_stencil_attachment: Option<(ImageHandle, Option<(f32, u32)>)>,
    ) {
        let pass = crate::render_graph::RenderPass2 {
            label_name: name,
            label_color: color,
            command: Some(RenderPassCommand::Raster {
                framebuffer: Framebuffer {
                    color_attachments: color_attachments
                        .iter()
                        .map(|(image, clear)| ColorAttachment {
                            image: self.get_image_index(*image),
                            clear: *clear,
                        })
                        .collect(),
                    depth_stencil_attachment: depth_stencil_attachment.map(|(image, clear)| {
                        DepthStencilAttachment {
                            image: self.get_image_index(image),
                            clear,
                        }
                    }),
                },
                draw_commands: vec![],
            }),
        };

        let pass_set = crate::render_graph::RenderPassSet {
            memory_barriers: vec![vk::MemoryBarrier2::builder()
                .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .src_access_mask(vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .dst_access_mask(vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE)
                .build()],
            buffer_barriers: vec![],
            image_barriers: vec![],
            render_passes: vec![pass],
        };

        if self.render_graph.command_buffers.is_empty() {
            self.render_graph
                .command_buffers
                .push(CommandBuffer::default());
        }

        self.render_graph.command_buffers[0]
            .render_pass_sets
            .push(pass_set);
    }

    pub fn build(mut self) -> CompiledRenderGraph {
        if let Some(command_buffer) = self.render_graph.command_buffers.get_mut(0) {
            for swapchain_index in 0..self.render_graph.swapchain_images.len() {
                command_buffer.command_buffer_wait_dependencies.push(
                    CommandBufferDependency::Swapchain {
                        index: swapchain_index,
                        stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
                    },
                );
                command_buffer.command_buffer_signal_dependencies.push(
                    CommandBufferDependency::Swapchain {
                        index: swapchain_index,
                        stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
                    },
                );
            }
        }

        self.render_graph
    }

    fn get_buffer_index(&mut self, buffer_handle: BufferHandle) -> BufferIndex {
        match self.buffer_index_map.get(&buffer_handle) {
            Some(index) => *index,
            None => {
                if let BufferHandle::Persistent(buffer_key) = buffer_handle {
                    let index = self.render_graph.buffer_resources.len() as BufferIndex;
                    self.render_graph
                        .buffer_resources
                        .push(BufferGraphResource {
                            description: BufferResourceDescription::Persistent(buffer_key),
                            last_access: BufferResourceAccess::None,
                        });
                    self.buffer_index_map.insert(buffer_handle, index);
                    index
                } else {
                    panic!("Invalid Transient Buffer Handle: {:?}", buffer_handle)
                }
            }
        }
    }

    fn get_buffer_offset(
        &mut self,
        buffer_offset: BufferOffset,
    ) -> crate::render_graph::BufferOffset {
        crate::render_graph::BufferOffset {
            buffer: self.get_buffer_index(buffer_offset.buffer),
            offset: buffer_offset.offset as u64,
        }
    }

    fn get_image_copy_buffer(
        &mut self,
        buffer: ImageCopyBuffer,
    ) -> crate::render_graph::ImageCopyBuffer {
        crate::render_graph::ImageCopyBuffer {
            buffer: self.get_buffer_index(buffer.buffer),
            offset: buffer.offset,
            row_length: buffer.row_length,
            row_height: buffer.row_height,
        }
    }

    pub fn get_image_index(&mut self, image_handle: ImageHandle) -> ImageIndex {
        match self.image_index_map.get(&image_handle) {
            Some(index) => *index,
            None => {
                if let ImageHandle::Persistent(image_key) = image_handle {
                    let index = self.render_graph.image_resources.len() as ImageIndex;
                    self.render_graph.image_resources.push(ImageGraphResource {
                        description: ImageResourceDescription::Persistent(image_key),
                        last_access: ImageResourceAccess::None,
                    });
                    self.image_index_map.insert(image_handle, index);
                    index
                } else {
                    panic!("Invalid Transient Image Handle: {:?}", image_handle)
                }
            }
        }
    }

    fn get_image_copy_image(
        &mut self,
        image: ImageCopyImage,
    ) -> crate::render_graph::ImageCopyImage {
        crate::render_graph::ImageCopyImage {
            image: self.get_image_index(image.image),
            offset: image.offset,
        }
    }
}
