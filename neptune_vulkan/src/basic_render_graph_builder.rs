use crate::render_graph::{
    BufferBarrier, BufferGraphResource, BufferIndex, BufferRead, BufferResourceDescription,
    BufferWrite, CommandBuffer, CommandBufferDependency, CompiledRenderGraph, Framebuffer,
    ImageBarrier, ImageBarrierSource, ImageGraphResource, ImageIndex, ImageResourceDescription,
    QueueType, RenderPassCommand,
};
use crate::render_graph_builder::{
    BufferOffset, ColorAttachment, ComputeDispatch, DepthStencilAttachment, DrawCommandDispatch,
    ImageCopyBuffer, ImageCopyImage, RasterDrawCommand, RenderGraphBuilderTrait,
};
use crate::render_graph_builder::{BufferReadCallback, BufferWriteCallback, ShaderResourceUsage};
use crate::resource_managers::{BufferResourceAccess, ImageResourceAccess};
use crate::{
    BufferHandle, BufferUsage, ComputePipelineHandle, ImageHandle, SurfaceHandle,
    TransientImageDesc,
};
use ash::vk;
use std::collections::HashMap;

#[derive(Debug)]
pub struct BasicRenderGraphBuilder {
    render_graph: CompiledRenderGraph,
    buffer_index_map: HashMap<BufferHandle, BufferIndex>,
    image_index_map: HashMap<ImageHandle, ImageIndex>,
}

impl Default for BasicRenderGraphBuilder {
    fn default() -> Self {
        let render_graph = CompiledRenderGraph {
            command_buffers: vec![CommandBuffer::default()],
            ..Default::default()
        };
        Self {
            render_graph,
            buffer_index_map: Default::default(),
            image_index_map: Default::default(),
        }
    }
}

impl RenderGraphBuilderTrait for BasicRenderGraphBuilder {
    fn add_buffer_write(
        &mut self,
        buffer_offset: BufferOffset,
        write_size: usize,
        callback: BufferWriteCallback,
    ) {
        let buffer_offset = self.get_buffer_offset(buffer_offset);
        self.render_graph.buffer_writes.push(BufferWrite {
            buffer_offset,
            write_size,
            callback,
        });
    }

    fn add_buffer_read(
        &mut self,
        buffer_offset: BufferOffset,
        read_size: usize,
        callback: BufferReadCallback,
    ) {
        let buffer_offset = self.get_buffer_offset(buffer_offset);
        self.render_graph.buffer_reads.push(BufferRead {
            buffer_offset,
            read_size,
            callback,
        });
    }

    fn create_transient_buffer(
        &mut self,
        size: usize,
        usage: BufferUsage,
        location: gpu_allocator::MemoryLocation,
    ) -> BufferHandle {
        let index = self.render_graph.buffer_resources.len() as BufferIndex;
        self.render_graph
            .buffer_resources
            .push(BufferGraphResource {
                description: BufferResourceDescription::Transient {
                    size,
                    usage,
                    location,
                },
                last_access: BufferResourceAccess::None,
            });
        let handle = BufferHandle::Transient(index);
        self.buffer_index_map.insert(handle, index);
        handle
    }

    fn create_transient_image(&mut self, desc: TransientImageDesc) -> ImageHandle {
        let index = self.render_graph.image_resources.len() as ImageIndex;
        self.render_graph.image_resources.push(ImageGraphResource {
            description: ImageResourceDescription::Transient(desc),
            first_access: None,
            last_access: None,
        });
        let handle = ImageHandle::Transient(index);
        self.image_index_map.insert(handle, index);
        handle
    }

    fn acquire_swapchain_image(&mut self, surface_handle: SurfaceHandle) -> ImageHandle {
        let index = self.render_graph.image_resources.len() as ImageIndex;
        let swapchain_index = self.render_graph.swapchain_images.len();
        self.render_graph
            .swapchain_images
            .push((surface_handle, index));
        self.render_graph.image_resources.push(ImageGraphResource {
            description: ImageResourceDescription::Swapchain(swapchain_index),
            first_access: None,
            last_access: None,
        });
        let handle = ImageHandle::Transient(index);
        self.image_index_map.insert(handle, index);
        handle
    }

    fn add_transfer_pass(
        &mut self,
        name: String,
        color: [f32; 4],
        queue: QueueType,
        transfers: &[crate::render_graph_builder::Transfer],
    ) {
        //TODO: queue
        let _ = queue;

        let mut buffer_usages = Vec::new();
        let mut image_usages = Vec::new();

        let transfers: Vec<crate::render_graph::Transfer> = transfers
            .iter()
            .map(|transfer| match transfer {
                crate::render_graph_builder::Transfer::CopyBufferToBuffer {
                    src,
                    dst,
                    copy_size,
                } => {
                    let src = self.get_buffer_offset(*src);
                    let dst = self.get_buffer_offset(*dst);
                    buffer_usages.push((src.buffer, BufferResourceAccess::TransferRead));
                    buffer_usages.push((dst.buffer, BufferResourceAccess::TransferWrite));
                    crate::render_graph::Transfer::BufferToBuffer {
                        src,
                        dst,
                        copy_size: *copy_size as vk::DeviceSize,
                    }
                }
                crate::render_graph_builder::Transfer::CopyBufferToImage {
                    src,
                    dst,
                    copy_size,
                } => {
                    let src = self.get_image_copy_buffer(*src);
                    let dst = self.get_image_copy_image(*dst);
                    buffer_usages.push((src.buffer, BufferResourceAccess::TransferRead));
                    image_usages.push((dst.image, ImageResourceAccess::TransferWrite));
                    crate::render_graph::Transfer::BufferToImage {
                        src,
                        dst,
                        copy_size: *copy_size,
                    }
                }
                crate::render_graph_builder::Transfer::CopyImageToBuffer {
                    src,
                    dst,
                    copy_size,
                } => {
                    let src = self.get_image_copy_image(*src);
                    let dst = self.get_image_copy_buffer(*dst);
                    image_usages.push((src.image, ImageResourceAccess::TransferRead));
                    buffer_usages.push((dst.buffer, BufferResourceAccess::TransferWrite));
                    crate::render_graph::Transfer::ImageToBuffer {
                        src,
                        dst,
                        copy_size: *copy_size,
                    }
                }
                crate::render_graph_builder::Transfer::CopyImageToImage {
                    src,
                    dst,
                    copy_size,
                } => {
                    let src = self.get_image_copy_image(*src);
                    let dst = self.get_image_copy_image(*dst);
                    image_usages.push((src.image, ImageResourceAccess::TransferRead));
                    image_usages.push((dst.image, ImageResourceAccess::TransferWrite));
                    crate::render_graph::Transfer::ImageToImage {
                        src,
                        dst,
                        copy_size: *copy_size,
                    }
                }
            })
            .collect();

        self.add_render_pass(
            name,
            color,
            &buffer_usages,
            &image_usages,
            Some(RenderPassCommand::Transfer { transfers }),
        );
    }

    fn add_compute_pass(
        &mut self,
        name: String,
        color: [f32; 4],
        queue: QueueType,
        pipeline: ComputePipelineHandle,
        dispatch: ComputeDispatch,
        resources: &[ShaderResourceUsage],
    ) {
        //TODO: queue
        let _ = queue;

        let mut buffer_usages = Vec::new();
        let mut image_usages = Vec::new();

        let resources =
            self.get_shader_resource_access(&mut buffer_usages, &mut image_usages, resources);

        let dispatch = match dispatch {
            ComputeDispatch::Size(size) => crate::render_graph::ComputeDispatch::Size(size),
            ComputeDispatch::Indirect(buffer_offset) => {
                let buffer_offset = self.get_buffer_offset(buffer_offset);
                buffer_usages.push((buffer_offset.buffer, BufferResourceAccess::IndirectRead));
                crate::render_graph::ComputeDispatch::Indirect(buffer_offset)
            }
        };

        self.add_render_pass(
            name,
            color,
            &buffer_usages,
            &image_usages,
            Some(RenderPassCommand::Compute {
                pipeline,
                resources,
                dispatch,
            }),
        );
    }

    fn add_raster_pass(
        &mut self,
        name: String,
        color: [f32; 4],
        color_attachments: &[ColorAttachment],
        depth_stencil_attachment: Option<DepthStencilAttachment>,
        raster_draw_commands: &[RasterDrawCommand],
    ) {
        let mut buffer_usages = Vec::new();
        let mut image_usages = Vec::new();

        let raster_command = RenderPassCommand::Raster {
            framebuffer: Framebuffer {
                color_attachments: color_attachments
                    .iter()
                    .map(|attachment| {
                        let image_index = self.get_image_index(attachment.image);
                        image_usages.push((image_index, ImageResourceAccess::AttachmentWrite));
                        crate::render_graph::ColorAttachment {
                            image: image_index,
                            clear: attachment.clear,
                        }
                    })
                    .collect(),
                depth_stencil_attachment: depth_stencil_attachment.map(|attachment| {
                    let image_index = self.get_image_index(attachment.image);
                    image_usages.push((image_index, ImageResourceAccess::AttachmentWrite));
                    crate::render_graph::DepthStencilAttachment {
                        image: image_index,
                        clear: attachment.clear,
                    }
                }),
            },
            draw_commands: self.get_raster_draw_commands(
                &mut buffer_usages,
                &mut image_usages,
                raster_draw_commands,
            ),
        };

        self.add_render_pass(name, color, &[], &image_usages, Some(raster_command));
    }

    fn build(mut self) -> CompiledRenderGraph {
        if let Some(command_buffer) = self.render_graph.command_buffers.get_mut(0) {
            for (swapchain_index, (_, image_index)) in
                self.render_graph.swapchain_images.iter().enumerate()
            {
                command_buffer.command_buffer_wait_dependencies.push(
                    CommandBufferDependency::Swapchain {
                        index: swapchain_index,
                        access: self.render_graph.image_resources[*image_index]
                            .first_access
                            .unwrap_or(ImageResourceAccess::None),
                    },
                );
                command_buffer.command_buffer_signal_dependencies.push(
                    CommandBufferDependency::Swapchain {
                        index: swapchain_index,
                        access: self.render_graph.image_resources[*image_index]
                            .last_access
                            .unwrap_or(ImageResourceAccess::None),
                    },
                );
            }
        }

        self.render_graph
    }
}

impl BasicRenderGraphBuilder {
    fn add_render_pass(
        &mut self,
        label_name: String,
        label_color: [f32; 4],
        buffer_usages: &[(BufferIndex, BufferResourceAccess)],
        image_usages: &[(ImageIndex, ImageResourceAccess)],
        command: Option<RenderPassCommand>,
    ) {
        let buffer_barriers = self.create_buffer_barriers(buffer_usages);
        let image_barriers = self.create_image_barriers(image_usages);
        self.render_graph.command_buffers[0].render_pass_sets.push(
            crate::render_graph::RenderPassSet {
                memory_barriers: vec![vk::MemoryBarrier2::builder()
                    .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .src_access_mask(vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .dst_access_mask(vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE)
                    .build()],
                buffer_barriers,
                image_barriers,
                render_passes: vec![crate::render_graph::RenderPass {
                    label_name,
                    label_color,
                    command,
                }],
            },
        );
    }

    fn create_buffer_barriers(
        &mut self,
        buffer_usages: &[(BufferIndex, BufferResourceAccess)],
    ) -> Vec<BufferBarrier> {
        //Unused since this uses global barriers
        let _ = buffer_usages;
        Vec::new()
    }

    fn create_image_barriers(
        &mut self,
        image_usages: &[(ImageIndex, ImageResourceAccess)],
    ) -> Vec<ImageBarrier> {
        image_usages
            .iter()
            .map(|(image_index, dst_access)| {
                //Update first access if it doesn't exist
                let _ = self.render_graph.image_resources[*image_index]
                    .first_access
                    .get_or_insert(*dst_access);
                let src = match self.render_graph.image_resources[*image_index]
                    .last_access
                    .replace(*dst_access)
                {
                    None => ImageBarrierSource::FirstUsage,
                    Some(access) => ImageBarrierSource::Precalculated(access),
                };
                ImageBarrier {
                    index: *image_index,
                    src,
                    dst: *dst_access,
                }
            })
            .collect()
    }

    fn get_raster_draw_commands(
        &mut self,
        buffer_usages: &mut Vec<(BufferIndex, BufferResourceAccess)>,
        image_usages: &mut Vec<(ImageIndex, ImageResourceAccess)>,
        raster_draw_commands: &[RasterDrawCommand],
    ) -> Vec<crate::render_graph::RasterDrawCommand> {
        raster_draw_commands
            .iter()
            .map(
                |raster_draw_command| crate::render_graph::RasterDrawCommand {
                    pipeline: raster_draw_command.pipeline,
                    vertex_buffers: raster_draw_command
                        .vertex_buffers
                        .iter()
                        .map(|vertex_buffer| {
                            let vertex_buffer = self.get_buffer_offset(*vertex_buffer);
                            buffer_usages
                                .push((vertex_buffer.buffer, BufferResourceAccess::VertexRead));
                            vertex_buffer
                        })
                        .collect(),
                    resources: self.get_shader_resource_access(
                        buffer_usages,
                        image_usages,
                        &raster_draw_command.resources,
                    ),
                    dispatch: match raster_draw_command.dispatch.clone() {
                        DrawCommandDispatch::Draw {
                            vertices,
                            instances,
                        } => crate::render_graph::DrawCommandDispatch::Draw {
                            vertices,
                            instances,
                        },
                        DrawCommandDispatch::DrawIndexed {
                            base_vertex,
                            indices,
                            instances,
                            index_buffer,
                            index_type,
                        } => {
                            let index_buffer = self.get_buffer_offset(index_buffer);
                            buffer_usages
                                .push((index_buffer.buffer, BufferResourceAccess::IndexRead));
                            crate::render_graph::DrawCommandDispatch::DrawIndexed {
                                base_vertex,
                                indices,
                                instances,
                                index_buffer,
                                index_type,
                            }
                        }
                        DrawCommandDispatch::DrawIndirect {
                            indirect_buffer,
                            draw_count,
                            stride,
                        } => {
                            let indirect_buffer = self.get_buffer_offset(indirect_buffer);
                            buffer_usages
                                .push((indirect_buffer.buffer, BufferResourceAccess::IndirectRead));
                            crate::render_graph::DrawCommandDispatch::DrawIndirect {
                                indirect_buffer,
                                draw_count,
                                stride,
                            }
                        }
                        DrawCommandDispatch::DrawIndirectIndexed {
                            indirect_buffer,
                            draw_count,
                            stride,
                            index_buffer,
                            index_type,
                        } => {
                            let index_buffer = self.get_buffer_offset(index_buffer);
                            let indirect_buffer = self.get_buffer_offset(indirect_buffer);
                            buffer_usages
                                .push((indirect_buffer.buffer, BufferResourceAccess::IndirectRead));
                            buffer_usages
                                .push((index_buffer.buffer, BufferResourceAccess::IndexRead));
                            crate::render_graph::DrawCommandDispatch::DrawIndirectIndexed {
                                indirect_buffer,
                                draw_count,
                                stride,
                                index_buffer,
                                index_type,
                            }
                        }
                    },
                },
            )
            .collect()
    }

    fn get_shader_resource_access(
        &mut self,
        buffer_usages: &mut Vec<(BufferIndex, BufferResourceAccess)>,
        image_usages: &mut Vec<(ImageIndex, ImageResourceAccess)>,
        resources: &[ShaderResourceUsage],
    ) -> Vec<crate::render_graph::ShaderResourceUsage> {
        resources
            .iter()
            .map(|resource| match resource {
                ShaderResourceUsage::StorageBuffer { buffer, write } => {
                    let buffer = self.get_buffer_index(*buffer);
                    buffer_usages.push((
                        buffer,
                        if *write {
                            BufferResourceAccess::StorageWrite
                        } else {
                            BufferResourceAccess::StorageRead
                        },
                    ));
                    crate::render_graph::ShaderResourceUsage::StorageBuffer {
                        buffer,
                        write: *write,
                    }
                }
                ShaderResourceUsage::StorageImage { image, write } => {
                    let image = self.get_image_index(*image);
                    image_usages.push((
                        image,
                        if *write {
                            ImageResourceAccess::StorageWrite
                        } else {
                            ImageResourceAccess::StorageRead
                        },
                    ));
                    crate::render_graph::ShaderResourceUsage::StorageImage {
                        image,
                        write: *write,
                    }
                }
                ShaderResourceUsage::SampledImage(image) => {
                    let image = self.get_image_index(*image);
                    image_usages.push((image, ImageResourceAccess::SampledRead));
                    crate::render_graph::ShaderResourceUsage::SampledImage(image)
                }
                ShaderResourceUsage::Sampler(sampler) => {
                    crate::render_graph::ShaderResourceUsage::Sampler(*sampler)
                }
            })
            .collect()
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
                        first_access: None,
                        last_access: None,
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
