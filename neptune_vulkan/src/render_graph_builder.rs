use crate::image::TransientImageDesc;
use crate::render_graph::{
    BufferIndex, BufferResourceDescription, BufferResourceUsage, ImageIndex,
    ImageResourceDescription, ImageResourceUsage, IndexType, QueueType, RenderGraph,
};
use crate::{
    BufferDescription, BufferHandle, ComputePipelineHandle, ImageHandle, RasterPipelineHandle,
    SamplerHandle, SurfaceHandle,
};
use std::collections::HashMap;
use std::ops::Range;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BufferOffset {
    pub buffer: BufferHandle,
    pub offset: usize,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct ImageCopyBuffer {
    pub buffer: BufferHandle,
    pub offset: u64,
    pub row_length: Option<u32>,
    pub row_height: Option<u32>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct ImageCopyImage {
    pub image: ImageHandle,
    pub offset: [u32; 2],
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Transfer {
    CopyBufferToBuffer {
        src: BufferOffset,
        dst: BufferOffset,
        copy_size: u64,
    },
    CopyBufferToImage {
        src: ImageCopyBuffer,
        dst: ImageCopyImage,
        copy_size: [u32; 2],
    },
    CopyImageToBuffer {
        src: ImageCopyImage,
        dst: ImageCopyBuffer,
        copy_size: [u32; 2],
    },
    CopyImageToImage {
        src: ImageCopyImage,
        dst: ImageCopyImage,
        copy_size: [u32; 2],
    },
}

#[derive(Debug)]
pub struct TransferPassBuilder {
    pub(crate) name: String,
    pub(crate) queue: QueueType,
    pub(crate) transfers: Vec<Transfer>,
}

impl TransferPassBuilder {
    pub fn new(name: &str, queue: QueueType) -> Self {
        Self {
            name: name.to_string(),
            queue,
            transfers: Vec::new(),
        }
    }

    pub fn build(self) -> RenderPass {
        RenderPass {
            label_name: self.name,
            label_color: [1.0, 0.0, 0.0, 1.0],
            queue: self.queue,
            pass_type: RenderPassType::Transfer {
                transfers: self.transfers,
            },
        }
    }
}

#[derive(Debug)]
pub enum ShaderResourceUsage {
    StorageBuffer { buffer: BufferHandle, write: bool },
    StorageImage { image: ImageHandle, write: bool },
    SampledImage(ImageHandle),
    Sampler(SamplerHandle),
}

#[derive(Debug, Eq, PartialEq)]
pub enum ComputeDispatch {
    Size([u32; 3]),
    Indirect(BufferOffset),
}

#[derive(Debug)]
pub struct ComputePassBuilder {
    name: String,
    queue: QueueType,
    pipeline: ComputePipelineHandle,
    resources: Vec<ShaderResourceUsage>,
    dispatch: ComputeDispatch,
}

impl ComputePassBuilder {
    pub fn new(name: &str, queue: QueueType, pipeline: ComputePipelineHandle) -> Self {
        Self {
            name: name.to_string(),
            queue,
            pipeline,
            resources: Vec::new(),
            dispatch: ComputeDispatch::Size([1; 3]),
        }
    }

    pub fn dispatch_size(mut self, size: [u32; 3]) -> Self {
        self.dispatch = ComputeDispatch::Size(size);
        self
    }

    pub fn dispatch_indirect(mut self, buffer: BufferHandle, offset: usize) -> Self {
        self.dispatch = ComputeDispatch::Indirect(BufferOffset { buffer, offset });
        self
    }

    pub fn read_buffer(mut self, buffer: BufferHandle) -> Self {
        self.resources.push(ShaderResourceUsage::StorageBuffer {
            buffer,
            write: false,
        });
        self
    }

    pub fn write_buffer(mut self, buffer: BufferHandle) -> Self {
        self.resources.push(ShaderResourceUsage::StorageBuffer {
            buffer,
            write: true,
        });
        self
    }

    pub fn read_storage_image(mut self, image: ImageHandle) -> Self {
        self.resources.push(ShaderResourceUsage::StorageImage {
            image,
            write: false,
        });
        self
    }

    pub fn write_storage_image(mut self, image: ImageHandle) -> Self {
        self.resources
            .push(ShaderResourceUsage::StorageImage { image, write: true });
        self
    }

    pub fn read_sampled_image(mut self, image: ImageHandle) -> Self {
        self.resources
            .push(ShaderResourceUsage::SampledImage(image));
        self
    }

    pub fn read_sampler(mut self, sampler: SamplerHandle) -> Self {
        self.resources.push(ShaderResourceUsage::Sampler(sampler));
        self
    }

    pub fn build(self) -> RenderPass {
        RenderPass {
            label_name: self.name,
            label_color: [0.0, 1.0, 0.0, 1.0],
            queue: self.queue,
            pass_type: RenderPassType::Compute {
                pipeline: self.pipeline,
                resources: self.resources,
                dispatch: self.dispatch,
            },
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
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

#[derive(Debug, PartialEq, Copy, Clone)]
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

#[derive(Default, Debug)]
pub struct Framebuffer {
    pub color_attachments: Vec<ColorAttachment>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RasterDispatch {
    Draw {
        vertices: Range<u32>,
        instances: Range<u32>,
    },
    DrawIndexed {
        base_vertex: i32,
        indices: Range<u32>,
        instances: Range<u32>,
    },
    DrawIndirect {
        buffer: BufferOffset,
        draw_count: u32,
        stride: u32,
    },
    DrawIndirectIndexed {
        buffer: BufferOffset,
        draw_count: u32,
        stride: u32,
    },
}

#[derive(Debug)]
pub struct RasterDrawCommand {
    pub pipeline: RasterPipelineHandle,
    pub vertex_buffers: Vec<BufferOffset>,
    pub index_buffer: Option<(BufferOffset, IndexType)>,
    pub resources: Vec<ShaderResourceUsage>,
    pub dispatch: RasterDispatch,
}

#[derive(Debug)]
pub struct RasterPassBuilder {
    name: String,
    framebuffer: Framebuffer,
    draw_commands: Vec<RasterDrawCommand>,
}

impl RasterPassBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            framebuffer: Framebuffer::default(),
            draw_commands: Vec::new(),
        }
    }

    pub fn add_color_attachment(mut self, attachment: ColorAttachment) -> Self {
        self.framebuffer.color_attachments.push(attachment);
        self
    }

    pub fn add_depth_stencil_attachment(mut self, attachment: DepthStencilAttachment) -> Self {
        self.framebuffer.depth_stencil_attachment = Some(attachment);
        self
    }

    pub fn add_draw_command(mut self, draw_command: RasterDrawCommand) -> Self {
        self.draw_commands.push(draw_command);
        self
    }

    pub fn build(self) -> RenderPass {
        RenderPass {
            label_name: self.name,
            label_color: [0.0, 0.0, 1.0, 1.0],
            queue: QueueType::Graphics,
            pass_type: RenderPassType::Raster {
                framebuffer: self.framebuffer,
                draw_commands: self.draw_commands,
            },
        }
    }
}

#[derive(Debug)]
pub(crate) enum RenderPassType {
    Transfer {
        transfers: Vec<Transfer>,
    },
    Compute {
        pipeline: ComputePipelineHandle,
        resources: Vec<ShaderResourceUsage>,
        dispatch: ComputeDispatch,
    },
    Raster {
        framebuffer: Framebuffer,
        draw_commands: Vec<RasterDrawCommand>,
    },
}

#[derive(Debug)]
pub struct BufferAccess {
    pub handle: BufferHandle,
    pub usage: BufferResourceUsage,
    //TODO: add access range
}

#[derive(Debug)]
pub struct ImageAccess {
    pub handle: ImageHandle,
    pub usage: ImageResourceUsage,
    //TODO: add access subresource range
}

fn write_shader_buffer_access(
    buffer_accesses: &mut Vec<BufferAccess>,
    resources: &[ShaderResourceUsage],
) {
    for resource in resources {
        if let ShaderResourceUsage::StorageBuffer { buffer, write } = resource {
            buffer_accesses.push(BufferAccess {
                handle: *buffer,
                usage: if *write {
                    BufferResourceUsage::StorageWrite
                } else {
                    BufferResourceUsage::StorageRead
                },
            });
        }
    }
}

fn write_shader_image_access(
    image_accesses: &mut Vec<ImageAccess>,
    resources: &[ShaderResourceUsage],
) {
    for resource in resources {
        match resource {
            ShaderResourceUsage::StorageImage { image, write } => {
                image_accesses.push(ImageAccess {
                    handle: *image,
                    usage: if *write {
                        ImageResourceUsage::StorageWrite
                    } else {
                        ImageResourceUsage::StorageRead
                    },
                });
            }
            ShaderResourceUsage::SampledImage(image) => {
                image_accesses.push(ImageAccess {
                    handle: *image,
                    usage: ImageResourceUsage::SampledRead,
                });
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct RenderPass {
    pub(crate) label_name: String,
    pub(crate) label_color: [f32; 4],
    pub(crate) queue: QueueType,
    pub(crate) pass_type: RenderPassType,
}

impl RenderPass {
    pub fn get_buffer_usages(&self) -> Vec<BufferAccess> {
        let mut buffer_accesses = Vec::new();
        match &self.pass_type {
            RenderPassType::Transfer { transfers } => {
                for transfer in transfers {
                    match transfer {
                        Transfer::CopyBufferToBuffer { src, dst, .. } => {
                            buffer_accesses.push(BufferAccess {
                                handle: src.buffer,
                                usage: BufferResourceUsage::TransferRead,
                            });

                            buffer_accesses.push(BufferAccess {
                                handle: dst.buffer,
                                usage: BufferResourceUsage::TransferWrite,
                            });
                        }
                        Transfer::CopyBufferToImage { src, .. } => {
                            buffer_accesses.push(BufferAccess {
                                handle: src.buffer,
                                usage: BufferResourceUsage::TransferRead,
                            });
                        }
                        Transfer::CopyImageToBuffer { dst, .. } => {
                            buffer_accesses.push(BufferAccess {
                                handle: dst.buffer,
                                usage: BufferResourceUsage::TransferWrite,
                            });
                        }
                        Transfer::CopyImageToImage { .. } => {}
                    }
                }
            }
            RenderPassType::Compute {
                resources,
                dispatch,
                ..
            } => {
                if let ComputeDispatch::Indirect(buffer) = dispatch {
                    buffer_accesses.push(BufferAccess {
                        handle: buffer.buffer,
                        usage: BufferResourceUsage::IndirectRead,
                    });
                }
                write_shader_buffer_access(&mut buffer_accesses, resources);
            }
            RenderPassType::Raster { draw_commands, .. } => {
                for draw_command in draw_commands {
                    for vertex_buffer in draw_command.vertex_buffers.iter() {
                        buffer_accesses.push(BufferAccess {
                            handle: vertex_buffer.buffer,
                            usage: BufferResourceUsage::VertexRead,
                        });
                    }

                    if let Some((index_buffer, _)) = &draw_command.index_buffer {
                        buffer_accesses.push(BufferAccess {
                            handle: index_buffer.buffer,
                            usage: BufferResourceUsage::IndexRead,
                        });
                    }

                    match draw_command.dispatch {
                        RasterDispatch::DrawIndirect { buffer, .. } => {
                            buffer_accesses.push(BufferAccess {
                                handle: buffer.buffer,
                                usage: BufferResourceUsage::IndirectRead,
                            });
                        }
                        RasterDispatch::DrawIndirectIndexed { buffer, .. } => {
                            buffer_accesses.push(BufferAccess {
                                handle: buffer.buffer,
                                usage: BufferResourceUsage::IndirectRead,
                            });
                        }
                        _ => {}
                    }

                    write_shader_buffer_access(&mut buffer_accesses, &draw_command.resources);
                }
            }
        }

        buffer_accesses
    }

    pub fn get_image_accesses(&self) -> Vec<ImageAccess> {
        let mut image_accesses = Vec::new();
        match &self.pass_type {
            RenderPassType::Transfer { transfers } => {
                for transfer in transfers {
                    match transfer {
                        Transfer::CopyBufferToBuffer { .. } => {}
                        Transfer::CopyBufferToImage { dst, .. } => {
                            image_accesses.push(ImageAccess {
                                handle: dst.image,
                                usage: ImageResourceUsage::TransferWrite,
                            });
                        }
                        Transfer::CopyImageToBuffer { src, .. } => {
                            image_accesses.push(ImageAccess {
                                handle: src.image,
                                usage: ImageResourceUsage::TransferRead,
                            });
                        }
                        Transfer::CopyImageToImage { src, dst, .. } => {
                            image_accesses.push(ImageAccess {
                                handle: src.image,
                                usage: ImageResourceUsage::TransferRead,
                            });

                            image_accesses.push(ImageAccess {
                                handle: dst.image,
                                usage: ImageResourceUsage::TransferWrite,
                            });
                        }
                    }
                }
            }
            RenderPassType::Compute { resources, .. } => {
                write_shader_image_access(&mut image_accesses, resources);
            }
            RenderPassType::Raster {
                framebuffer,
                draw_commands,
            } => {
                for color_attachment in framebuffer.color_attachments.iter() {
                    image_accesses.push(ImageAccess {
                        handle: color_attachment.image,
                        usage: ImageResourceUsage::AttachmentWrite,
                    });
                }

                if let Some(depth_stencil_attachment) = framebuffer.depth_stencil_attachment {
                    image_accesses.push(ImageAccess {
                        handle: depth_stencil_attachment.image,
                        usage: ImageResourceUsage::AttachmentWrite,
                    });
                }

                for draw_command in draw_commands {
                    write_shader_image_access(&mut image_accesses, &draw_command.resources);
                }
            }
        }

        image_accesses
    }
}

#[derive(Default, Debug)]
pub struct RenderGraphBuilder {
    pub(crate) transient_buffers: Vec<BufferDescription>,
    pub(crate) transient_images: Vec<TransientImageDesc>,
    pub(crate) swapchain_images: Vec<SurfaceHandle>,
    pub(crate) passes: Vec<RenderPass>,
}

impl RenderGraphBuilder {
    pub fn create_transient_buffer(&mut self, desc: BufferDescription) -> BufferHandle {
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

    pub fn build(self) -> RenderGraph {
        RenderGraphIntermediate::convert(self)
    }
}

/// A set of function to convert the current render graph layout to the struct better suited to executing
/// TODO: create final format inplace rather than create than convert
#[derive(Default)]
struct RenderGraphIntermediate {
    transient_buffers: Vec<BufferDescription>,
    transient_images: Vec<TransientImageDesc>,
    swapchain_images: Vec<SurfaceHandle>,

    render_graph: RenderGraph,
    buffer_index_map: HashMap<BufferHandle, BufferIndex>,
    image_index_map: HashMap<ImageHandle, ImageIndex>,
}

impl RenderGraphIntermediate {
    fn convert(rgb: RenderGraphBuilder) -> RenderGraph {
        let mut rgi = Self {
            transient_buffers: rgb.transient_buffers,
            transient_images: rgb.transient_images,
            swapchain_images: rgb.swapchain_images,
            render_graph: Default::default(),
            buffer_index_map: Default::default(),
            image_index_map: Default::default(),
        };

        for pass in rgb.passes {
            rgi.add_pass(pass);
        }

        rgi.render_graph
    }

    fn get_buffer_index(&mut self, handle: BufferHandle) -> BufferIndex {
        if let Some(index) = self.buffer_index_map.get(&handle) {
            *index
        } else {
            let index = self.render_graph.buffer_descriptions.len() as BufferIndex;
            self.render_graph.buffer_descriptions.push(match handle {
                BufferHandle::Persistent(key) => BufferResourceDescription::Persistent(key),
                BufferHandle::Transient(index) => {
                    BufferResourceDescription::Transient(self.transient_buffers[index].clone())
                }
            });
            self.buffer_index_map.insert(handle, index);
            index
        }
    }

    fn get_image_index(&mut self, handle: ImageHandle) -> ImageIndex {
        if let Some(index) = self.image_index_map.get(&handle) {
            *index
        } else {
            let index = self.render_graph.image_descriptions.len() as ImageIndex;
            self.render_graph.image_descriptions.push(match handle {
                ImageHandle::Persistent(key) => ImageResourceDescription::Persistent(key),
                ImageHandle::Transient(index) => {
                    ImageResourceDescription::Transient(self.transient_images[index].clone())
                }
                ImageHandle::Swapchain(index) => {
                    self.render_graph
                        .swapchain_images
                        .push((self.swapchain_images[index], index));
                    ImageResourceDescription::Swapchain(index)
                }
            });
            self.image_index_map.insert(handle, index);
            index
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

    fn get_image_copy_image(
        &mut self,
        image: ImageCopyImage,
    ) -> crate::render_graph::ImageCopyImage {
        crate::render_graph::ImageCopyImage {
            image: self.get_image_index(image.image),
            offset: image.offset,
        }
    }

    fn add_pass(&mut self, pass: RenderPass) {
        let mut buffer_usages = Vec::new();
        let mut image_usages = Vec::new();

        let command = match pass.pass_type {
            RenderPassType::Transfer { transfers } => {
                self.convert_transfer_pass(&mut buffer_usages, &mut image_usages, transfers)
            }
            RenderPassType::Compute {
                pipeline,
                resources,
                dispatch,
            } => self.convert_compute_pass(
                &mut buffer_usages,
                &mut image_usages,
                pipeline,
                resources,
                dispatch,
            ),
            RenderPassType::Raster {
                framebuffer,
                draw_commands,
            } => self.convert_raster_pass(
                &mut buffer_usages,
                &mut image_usages,
                framebuffer,
                draw_commands,
            ),
        };

        self.render_graph
            .render_passes
            .push(crate::render_graph::RenderPass {
                label_name: pass.label_name,
                label_color: pass.label_color,
                queue: pass.queue,
                buffer_usages,
                image_usages,
                command: Some(command),
            });
    }

    fn convert_transfer_pass(
        &mut self,
        buffer_usages: &mut Vec<(BufferIndex, BufferResourceUsage)>,
        image_usages: &mut Vec<(ImageIndex, ImageResourceUsage)>,
        mut transfers: Vec<Transfer>,
    ) -> crate::render_graph::RenderPassCommand {
        crate::render_graph::RenderPassCommand::Transfer {
            transfers: transfers
                .drain(..)
                .map(|transfer| match transfer {
                    Transfer::CopyBufferToBuffer {
                        src,
                        dst,
                        copy_size,
                    } => {
                        let src = self.get_buffer_offset(src);
                        let dst = self.get_buffer_offset(dst);
                        buffer_usages.push((src.buffer, BufferResourceUsage::TransferRead));
                        buffer_usages.push((dst.buffer, BufferResourceUsage::TransferWrite));
                        crate::render_graph::Transfer::BufferToBuffer {
                            src,
                            dst,
                            copy_size,
                        }
                    }
                    Transfer::CopyBufferToImage {
                        src,
                        dst,
                        copy_size,
                    } => {
                        let src = self.get_image_copy_buffer(src);
                        let dst = self.get_image_copy_image(dst);
                        buffer_usages.push((src.buffer, BufferResourceUsage::TransferRead));
                        image_usages.push((dst.image, ImageResourceUsage::TransferWrite));
                        crate::render_graph::Transfer::BufferToImage {
                            src,
                            dst,
                            copy_size,
                        }
                    }
                    Transfer::CopyImageToBuffer {
                        src,
                        dst,
                        copy_size,
                    } => {
                        let src = self.get_image_copy_image(src);
                        let dst = self.get_image_copy_buffer(dst);
                        image_usages.push((src.image, ImageResourceUsage::TransferRead));
                        buffer_usages.push((dst.buffer, BufferResourceUsage::TransferWrite));
                        crate::render_graph::Transfer::ImageToBuffer {
                            src,
                            dst,
                            copy_size,
                        }
                    }
                    Transfer::CopyImageToImage {
                        src,
                        dst,
                        copy_size,
                    } => {
                        let src = self.get_image_copy_image(src);
                        let dst = self.get_image_copy_image(dst);
                        image_usages.push((src.image, ImageResourceUsage::TransferRead));
                        image_usages.push((dst.image, ImageResourceUsage::TransferWrite));
                        crate::render_graph::Transfer::ImageToImage {
                            src,
                            dst,
                            copy_size,
                        }
                    }
                })
                .collect(),
        }
    }

    fn convert_compute_pass(
        &mut self,
        buffer_usages: &mut Vec<(BufferIndex, BufferResourceUsage)>,
        image_usages: &mut Vec<(ImageIndex, ImageResourceUsage)>,
        pipeline: ComputePipelineHandle,
        resources: Vec<ShaderResourceUsage>,
        dispatch: ComputeDispatch,
    ) -> crate::render_graph::RenderPassCommand {
        crate::render_graph::RenderPassCommand::Compute {
            pipeline,
            resources: self.convert_shader_resource(buffer_usages, image_usages, resources),
            dispatch: match dispatch {
                ComputeDispatch::Size(size) => crate::render_graph::ComputeDispatch::Size(size),
                ComputeDispatch::Indirect(buffer_offset) => {
                    let buffer_offset = self.get_buffer_offset(buffer_offset);
                    buffer_usages.push((buffer_offset.buffer, BufferResourceUsage::IndirectRead));
                    crate::render_graph::ComputeDispatch::Indirect(buffer_offset)
                }
            },
        }
    }

    fn convert_raster_pass(
        &mut self,
        buffer_usages: &mut Vec<(BufferIndex, BufferResourceUsage)>,
        image_usages: &mut Vec<(ImageIndex, ImageResourceUsage)>,
        mut framebuffer: Framebuffer,
        mut draw_command: Vec<RasterDrawCommand>,
    ) -> crate::render_graph::RenderPassCommand {
        crate::render_graph::RenderPassCommand::Raster {
            framebuffer: crate::render_graph::Framebuffer {
                color_attachments: framebuffer
                    .color_attachments
                    .drain(..)
                    .map(|attachment| {
                        let image = self.get_image_index(attachment.image);
                        image_usages.push((image, ImageResourceUsage::AttachmentWrite));
                        crate::render_graph::ColorAttachment {
                            image,
                            clear: attachment.clear,
                        }
                    })
                    .collect(),
                depth_stencil_attachment: framebuffer.depth_stencil_attachment.map(|attachment| {
                    let image = self.get_image_index(attachment.image);
                    image_usages.push((image, ImageResourceUsage::AttachmentWrite));
                    crate::render_graph::DepthStencilAttachment {
                        image,
                        clear: attachment.clear,
                    }
                }),
            },
            draw_commands: draw_command
                .drain(..)
                .map(|mut draw_command| crate::render_graph::RasterDrawCommand {
                    pipeline: draw_command.pipeline,
                    vertex_buffers: draw_command
                        .vertex_buffers
                        .drain(..)
                        .map(|vertex_buffer| {
                            let vertex_buffer = self.get_buffer_offset(vertex_buffer);
                            buffer_usages
                                .push((vertex_buffer.buffer, BufferResourceUsage::VertexRead));
                            vertex_buffer
                        })
                        .collect(),
                    index_buffer: draw_command.index_buffer.map(|(index_buffer, index_type)| {
                        let index_buffer = self.get_buffer_offset(index_buffer);
                        buffer_usages.push((index_buffer.buffer, BufferResourceUsage::IndexRead));
                        (index_buffer, index_type)
                    }),
                    resources: self.convert_shader_resource(
                        buffer_usages,
                        image_usages,
                        draw_command.resources,
                    ),
                    dispatch: match draw_command.dispatch {
                        RasterDispatch::Draw {
                            vertices,
                            instances,
                        } => crate::render_graph::RasterDispatch::Draw {
                            vertices,
                            instances,
                        },
                        RasterDispatch::DrawIndexed {
                            base_vertex,
                            indices,
                            instances,
                        } => crate::render_graph::RasterDispatch::DrawIndexed {
                            base_vertex,
                            indices,
                            instances,
                        },
                        RasterDispatch::DrawIndirect {
                            buffer,
                            draw_count,
                            stride,
                        } => crate::render_graph::RasterDispatch::DrawIndirect {
                            buffer: {
                                let buffer = self.get_buffer_offset(buffer);
                                buffer_usages
                                    .push((buffer.buffer, BufferResourceUsage::IndirectRead));
                                buffer
                            },
                            draw_count,
                            stride,
                        },
                        RasterDispatch::DrawIndirectIndexed {
                            buffer,
                            draw_count,
                            stride,
                        } => crate::render_graph::RasterDispatch::DrawIndirectIndexed {
                            buffer: {
                                let buffer = self.get_buffer_offset(buffer);
                                buffer_usages
                                    .push((buffer.buffer, BufferResourceUsage::IndirectRead));
                                buffer
                            },
                            draw_count,
                            stride,
                        },
                    },
                })
                .collect(),
        }
    }

    fn convert_shader_resource(
        &mut self,
        buffer_usages: &mut Vec<(BufferIndex, BufferResourceUsage)>,
        image_usages: &mut Vec<(ImageIndex, ImageResourceUsage)>,
        mut resources: Vec<ShaderResourceUsage>,
    ) -> Vec<crate::render_graph::ShaderResourceUsage> {
        resources
            .drain(..)
            .map(|usage| match usage {
                ShaderResourceUsage::StorageBuffer { buffer, write } => {
                    let buffer = self.get_buffer_index(buffer);
                    buffer_usages.push((
                        buffer,
                        if write {
                            BufferResourceUsage::StorageWrite
                        } else {
                            BufferResourceUsage::StorageRead
                        },
                    ));
                    crate::render_graph::ShaderResourceUsage::StorageBuffer { buffer, write }
                }
                ShaderResourceUsage::StorageImage { image, write } => {
                    let image = self.get_image_index(image);
                    image_usages.push((
                        image,
                        if write {
                            ImageResourceUsage::StorageWrite
                        } else {
                            ImageResourceUsage::StorageRead
                        },
                    ));
                    crate::render_graph::ShaderResourceUsage::StorageImage { image, write }
                }
                ShaderResourceUsage::SampledImage(image) => {
                    let image = self.get_image_index(image);
                    image_usages.push((image, ImageResourceUsage::SampledRead));
                    crate::render_graph::ShaderResourceUsage::SampledImage(image)
                }
                ShaderResourceUsage::Sampler(sampler) => {
                    crate::render_graph::ShaderResourceUsage::Sampler(sampler)
                }
            })
            .collect()
    }
}

// #[derive(Default, Debug)]
// pub struct RenderGraphBuilder2 {
//     transient_buffer_descriptions: Vec<BufferDescription>,
//     transient_image_descriptions: Vec<TransientImageDesc>,
//     swapchain_images: Vec<SurfaceHandle>,
//
//     buffer_index_map: HashMap<BufferHandle, BufferIndex>,
//     image_index_map: HashMap<ImageHandle, ImageIndex>,
//     render_graph: RenderGraph,
// }
//
// impl RenderGraphBuilder2 {
//     pub fn create_transient_buffer(&mut self, desc: BufferDescription) -> BufferHandle {
//         let index = self.transient_buffer_descriptions.len();
//         self.transient_buffer_descriptions.push(desc);
//         BufferHandle::Transient(index)
//     }
//
//     pub fn create_transient_image(&mut self, desc: TransientImageDesc) -> ImageHandle {
//         let index = self.transient_image_descriptions.len();
//         self.transient_image_descriptions.push(desc);
//         ImageHandle::Transient(index)
//     }
//
//     pub fn acquire_swapchain_image(&mut self, surface_handle: SurfaceHandle) -> ImageHandle {
//         let index = self.swapchain_images.len();
//         self.swapchain_images.push(surface_handle);
//         ImageHandle::Swapchain(index)
//     }
//
//     pub fn create_transfer_pass(&mut self, name: &str, queue: QueueType) -> TransferPassBuilder2 {
//         TransferPassBuilder2 {
//             render_graph_builder: self,
//             name: name.to_string(),
//             queue,
//             transfers: Vec::new(),
//             buffer_usages: Vec::new(),
//             image_usages: Vec::new(),
//         }
//     }
//
//     pub fn create_compute_pass(
//         &mut self,
//         name: &str,
//         queue: QueueType,
//         pipeline: ComputePipelineHandle,
//     ) -> ComputePassBuilder2 {
//         ComputePassBuilder2 {
//             render_graph_builder: self,
//             name: name.to_string(),
//             queue,
//             pipeline,
//             resources: Vec::new(),
//             dispatch: crate::render_graph::ComputeDispatch::Size([1; 3]),
//             buffer_usages: Vec::new(),
//             image_usages: Vec::new(),
//         }
//     }
//
//     fn get_buffer_index(&mut self, handle: BufferHandle) -> BufferIndex {
//         if let Some(index) = self.buffer_index_map.get(&handle) {
//             *index
//         } else {
//             let index = self.render_graph.buffers.len() as BufferIndex;
//             self.render_graph.buffers.push(match handle {
//                 BufferHandle::Persistent(key) => BufferResource::Persistent(key),
//                 BufferHandle::Transient(index) => {
//                     BufferResource::Transient(self.transient_buffer_descriptions[index].clone())
//                 }
//             });
//             self.buffer_index_map.insert(handle, index);
//             index
//         }
//     }
//
//     fn get_image_index(&mut self, handle: ImageHandle) -> ImageIndex {
//         if let Some(index) = self.image_index_map.get(&handle) {
//             *index
//         } else {
//             let index = self.render_graph.images.len() as ImageIndex;
//             self.render_graph.images.push(match handle {
//                 ImageHandle::Persistent(key) => ImageResource::Persistent(key),
//                 ImageHandle::Transient(index) => {
//                     ImageResource::Transient(self.transient_image_descriptions[index].clone())
//                 }
//                 ImageHandle::Swapchain(index) => {
//                     ImageResource::Swapchain(self.swapchain_images[index])
//                 }
//             });
//             self.image_index_map.insert(handle, index);
//             index
//         }
//     }
//
//     fn get_buffer_offset(
//         &mut self,
//         buffer_offset: BufferOffset,
//     ) -> crate::render_graph::BufferOffset {
//         crate::render_graph::BufferOffset {
//             buffer: self.get_buffer_index(buffer_offset.buffer),
//             offset: buffer_offset.offset as u64,
//         }
//     }
//
//     fn get_image_copy_buffer(
//         &mut self,
//         buffer: ImageCopyBuffer,
//     ) -> crate::render_graph::ImageCopyBuffer {
//         crate::render_graph::ImageCopyBuffer {
//             buffer: self.get_buffer_index(buffer.buffer),
//             offset: buffer.offset,
//             row_length: buffer.row_length,
//             row_height: buffer.row_height,
//         }
//     }
//
//     fn get_image_copy_image(
//         &mut self,
//         image: ImageCopyImage,
//     ) -> crate::render_graph::ImageCopyImage {
//         crate::render_graph::ImageCopyImage {
//             image: self.get_image_index(image.image),
//             offset: image.offset,
//         }
//     }
// }
//
// pub struct TransferPassBuilder2<'a> {
//     render_graph_builder: &'a mut RenderGraphBuilder2,
//     name: String,
//     queue: QueueType,
//     transfers: Vec<crate::render_graph::Transfer>,
//     buffer_usages: Vec<(BufferIndex, BufferResourceUsage)>,
//     image_usages: Vec<(ImageIndex, ImageResourceUsage)>,
// }
//
// impl<'a> TransferPassBuilder2<'a> {
//     pub fn copy_buffer_to_buffer(
//         &mut self,
//         src: BufferOffset,
//         dst: BufferOffset,
//         copy_size: usize,
//     ) {
//         let src = self.render_graph_builder.get_buffer_offset(src);
//         let dst = self.render_graph_builder.get_buffer_offset(dst);
//
//         self.buffer_usages
//             .push((src.buffer, BufferResourceUsage::TransferRead));
//         self.buffer_usages
//             .push((dst.buffer, BufferResourceUsage::TransferWrite));
//
//         self.transfers
//             .push(crate::render_graph::Transfer::BufferToBuffer {
//                 src,
//                 dst,
//                 copy_size: copy_size as u64,
//             })
//     }
//
//     pub fn copy_buffer_to_image(
//         &mut self,
//         src: ImageCopyBuffer,
//         dst: ImageCopyImage,
//         copy_size: [u32; 2],
//     ) {
//         let src = self.render_graph_builder.get_image_copy_buffer(src);
//         let dst = self.render_graph_builder.get_image_copy_image(dst);
//
//         self.buffer_usages
//             .push((src.buffer, BufferResourceUsage::TransferRead));
//         self.image_usages
//             .push((dst.image, ImageResourceUsage::TransferWrite));
//
//         self.transfers
//             .push(crate::render_graph::Transfer::BufferToImage {
//                 src,
//                 dst,
//                 copy_size,
//             })
//     }
//
//     pub fn copy_image_to_buffer(
//         &mut self,
//         src: ImageCopyImage,
//         dst: ImageCopyBuffer,
//         copy_size: [u32; 2],
//     ) {
//         let src = self.render_graph_builder.get_image_copy_image(src);
//         let dst = self.render_graph_builder.get_image_copy_buffer(dst);
//
//         self.image_usages
//             .push((src.image, ImageResourceUsage::TransferRead));
//         self.buffer_usages
//             .push((dst.buffer, BufferResourceUsage::TransferWrite));
//
//         self.transfers
//             .push(crate::render_graph::Transfer::ImageToBuffer {
//                 src,
//                 dst,
//                 copy_size,
//             })
//     }
//
//     pub fn copy_image_to_image(
//         &mut self,
//         src: ImageCopyImage,
//         dst: ImageCopyImage,
//         copy_size: [u32; 2],
//     ) {
//         let src = self.render_graph_builder.get_image_copy_image(src);
//         let dst = self.render_graph_builder.get_image_copy_image(dst);
//
//         self.image_usages
//             .push((src.image, ImageResourceUsage::TransferRead));
//         self.image_usages
//             .push((dst.image, ImageResourceUsage::TransferWrite));
//
//         self.transfers
//             .push(crate::render_graph::Transfer::ImageToImage {
//                 src,
//                 dst,
//                 copy_size,
//             })
//     }
//
//     pub fn build(self) {
//         drop(self)
//     }
// }
//
// impl<'a> Drop for TransferPassBuilder2<'a> {
//     fn drop(&mut self) {
//         self.render_graph_builder
//             .render_graph
//             .render_pass
//             .push(crate::render_graph::RenderPass {
//                 label_name: std::mem::take(&mut self.name),
//                 label_color: [1.0, 0.0, 0.0, 1.0],
//                 queue: self.queue,
//                 buffer_usages: std::mem::take(&mut self.buffer_usages),
//                 image_usages: std::mem::take(&mut self.image_usages),
//                 command: Some(crate::render_graph::RenderPassCommand::Transfer {
//                     transfers: std::mem::take(&mut self.transfers),
//                 }),
//             });
//     }
// }
//
// #[derive(Debug)]
// pub struct ComputePassBuilder2<'a> {
//     render_graph_builder: &'a mut RenderGraphBuilder2,
//     name: String,
//     queue: QueueType,
//     pipeline: ComputePipelineHandle,
//     resources: Vec<crate::render_graph::ShaderResourceUsage>,
//     dispatch: crate::render_graph::ComputeDispatch,
//     buffer_usages: Vec<(BufferIndex, BufferResourceUsage)>,
//     image_usages: Vec<(ImageIndex, ImageResourceUsage)>,
// }
//
// impl<'a> ComputePassBuilder2<'a> {
//     pub fn dispatch_size(mut self, size: [u32; 3]) -> Self {
//         self.dispatch = crate::render_graph::ComputeDispatch::Size(size);
//         self
//     }
//
//     pub fn dispatch_indirect(mut self, buffer: BufferHandle, offset: usize) -> Self {
//         let buffer = self.render_graph_builder.get_buffer_index(buffer);
//         let offset = offset as u64;
//         self.buffer_usages
//             .push((buffer, BufferResourceUsage::IndirectRead));
//         self.dispatch =
//             crate::render_graph::ComputeDispatch::Indirect(crate::render_graph::BufferOffset {
//                 buffer,
//                 offset,
//             });
//         self
//     }
//
//     pub fn read_buffer(mut self, buffer: BufferHandle) -> Self {
//         let buffer = self.render_graph_builder.get_buffer_index(buffer);
//         self.buffer_usages
//             .push((buffer, BufferResourceUsage::StorageRead));
//         self.resources
//             .push(crate::render_graph::ShaderResourceUsage::StorageBuffer {
//                 buffer,
//                 write: false,
//             });
//         self
//     }
//
//     pub fn write_buffer(mut self, buffer: BufferHandle) -> Self {
//         let buffer = self.render_graph_builder.get_buffer_index(buffer);
//         self.buffer_usages
//             .push((buffer, BufferResourceUsage::StorageWrite));
//         self.resources
//             .push(crate::render_graph::ShaderResourceUsage::StorageBuffer {
//                 buffer,
//                 write: true,
//             });
//         self
//     }
//
//     pub fn read_storage_image(mut self, image: ImageHandle) -> Self {
//         let image = self.render_graph_builder.get_image_index(image);
//         self.image_usages
//             .push((image, ImageResourceUsage::StorageRead));
//         self.resources
//             .push(crate::render_graph::ShaderResourceUsage::StorageImage {
//                 image,
//                 write: false,
//             });
//         self
//     }
//
//     pub fn write_storage_image(mut self, image: ImageHandle) -> Self {
//         let image = self.render_graph_builder.get_image_index(image);
//         self.image_usages
//             .push((image, ImageResourceUsage::StorageWrite));
//         self.resources
//             .push(crate::render_graph::ShaderResourceUsage::StorageImage { image, write: true });
//         self
//     }
//
//     pub fn read_sampled_image(mut self, image: ImageHandle) -> Self {
//         let image = self.render_graph_builder.get_image_index(image);
//         self.image_usages
//             .push((image, ImageResourceUsage::SampledRead));
//         self.resources
//             .push(crate::render_graph::ShaderResourceUsage::SampledImage(
//                 image,
//             ));
//         self
//     }
//
//     pub fn read_sampler(mut self, sampler: SamplerHandle) -> Self {
//         self.resources
//             .push(crate::render_graph::ShaderResourceUsage::Sampler(sampler));
//         self
//     }
//
//     pub fn build(self) {
//         drop(self);
//     }
// }
//
// impl<'a> Drop for ComputePassBuilder2<'a> {
//     fn drop(&mut self) {
//         self.render_graph_builder
//             .render_graph
//             .render_pass
//             .push(crate::render_graph::RenderPass {
//                 label_name: std::mem::take(&mut self.name),
//                 label_color: [0.0, 1.0, 0.0, 1.0],
//                 queue: self.queue,
//                 buffer_usages: std::mem::take(&mut self.buffer_usages),
//                 image_usages: std::mem::take(&mut self.image_usages),
//                 command: Some(crate::render_graph::RenderPassCommand::Compute {
//                     pipeline: self.pipeline,
//                     resources: std::mem::take(&mut self.resources),
//                     dispatch: self.dispatch.clone(),
//                 }),
//             });
//     }
// }
