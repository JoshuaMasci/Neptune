use crate::image::TransientImageDesc;
use crate::render_graph::{
    BufferIndex, BufferGraphResource, BufferResourceDescription, ImageIndex, ImageGraphResource,
    ImageResourceDescription, IndexType, QueueType, RenderGraph, RenderPassCommand,
};
use crate::resource_managers::{BufferResourceAccess, ImageResourceAccess};
use crate::{
    BufferDescription, BufferHandle, ComputePipelineHandle, ImageHandle, RasterPipelineHandle,
    SamplerHandle, SurfaceHandle,
};
use log::info;
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

pub struct TransferPassBuilder2<'a> {
    render_graph_builder: &'a mut RenderGraphBuilder,
    name: String,
    color: [f32; 4],
    queue: QueueType,

    transfers: Vec<crate::render_graph::Transfer>,

    buffer_usages: Vec<(BufferIndex, BufferResourceAccess)>,
    image_usages: Vec<(ImageIndex, ImageResourceAccess)>,
}

impl<'a> TransferPassBuilder2<'a> {
    pub fn new(
        render_graph_builder: &'a mut RenderGraphBuilder,
        name: &str,
        queue: QueueType,
    ) -> Self {
        Self {
            render_graph_builder,
            name: name.to_string(),
            color: [1.0, 0.0, 0.0, 1.0],
            queue,
            transfers: Vec::new(),
            buffer_usages: Vec::new(),
            image_usages: Vec::new(),
        }
    }

    pub fn override_label_color(&mut self, color: [f32; 4]) {
        self.color = color;
    }

    pub fn copy_buffer_to_buffer(
        &mut self,
        src: BufferOffset,
        dst: BufferOffset,
        copy_size: usize,
    ) {
        let src = self.render_graph_builder.get_buffer_offset(src);
        let dst = self.render_graph_builder.get_buffer_offset(dst);

        self.buffer_usages
            .push((src.buffer, BufferResourceAccess::TransferRead));
        self.buffer_usages
            .push((dst.buffer, BufferResourceAccess::TransferWrite));

        self.transfers
            .push(crate::render_graph::Transfer::BufferToBuffer {
                src,
                dst,
                copy_size: copy_size as u64,
            })
    }

    pub fn copy_buffer_to_image(
        &mut self,
        src: ImageCopyBuffer,
        dst: ImageCopyImage,
        copy_size: [u32; 2],
    ) {
        let src = self.render_graph_builder.get_image_copy_buffer(src);
        let dst = self.render_graph_builder.get_image_copy_image(dst);

        self.buffer_usages
            .push((src.buffer, BufferResourceAccess::TransferRead));
        self.image_usages
            .push((dst.image, ImageResourceAccess::TransferWrite));

        self.transfers
            .push(crate::render_graph::Transfer::BufferToImage {
                src,
                dst,
                copy_size,
            })
    }

    pub fn copy_image_to_buffer(
        &mut self,
        src: ImageCopyImage,
        dst: ImageCopyBuffer,
        copy_size: [u32; 2],
    ) {
        let src = self.render_graph_builder.get_image_copy_image(src);
        let dst = self.render_graph_builder.get_image_copy_buffer(dst);

        self.image_usages
            .push((src.image, ImageResourceAccess::TransferRead));
        self.buffer_usages
            .push((dst.buffer, BufferResourceAccess::TransferWrite));

        self.transfers
            .push(crate::render_graph::Transfer::ImageToBuffer {
                src,
                dst,
                copy_size,
            })
    }

    pub fn copy_image_to_image(
        &mut self,
        src: ImageCopyImage,
        dst: ImageCopyImage,
        copy_size: [u32; 2],
    ) {
        let src = self.render_graph_builder.get_image_copy_image(src);
        let dst = self.render_graph_builder.get_image_copy_image(dst);

        self.image_usages
            .push((src.image, ImageResourceAccess::TransferRead));
        self.image_usages
            .push((dst.image, ImageResourceAccess::TransferWrite));

        self.transfers
            .push(crate::render_graph::Transfer::ImageToImage {
                src,
                dst,
                copy_size,
            })
    }

    pub fn build(self) {
        drop(self)
    }
}

impl<'a> Drop for TransferPassBuilder2<'a> {
    fn drop(&mut self) {
        self.render_graph_builder.render_graph.render_passes.push(
            crate::render_graph::RenderPass {
                label_name: std::mem::take(&mut self.name),
                label_color: self.color,
                queue: self.queue,
                buffer_usages: std::mem::take(&mut self.buffer_usages),
                image_usages: std::mem::take(&mut self.image_usages),
                command: Some(RenderPassCommand::Transfer {
                    transfers: std::mem::take(&mut self.transfers),
                }),
            },
        );
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
pub struct ComputePassBuilder<'a> {
    render_graph_builder: &'a mut RenderGraphBuilder,
    name: String,
    color: [f32; 4],
    queue: QueueType,

    pipeline: ComputePipelineHandle,
    resources: Vec<crate::render_graph::ShaderResourceUsage>,
    dispatch: crate::render_graph::ComputeDispatch,

    buffer_usages: Vec<(BufferIndex, BufferResourceAccess)>,
    image_usages: Vec<(ImageIndex, ImageResourceAccess)>,
}

impl<'a> ComputePassBuilder<'a> {
    pub fn new(
        render_graph_builder: &'a mut RenderGraphBuilder,
        name: &str,
        queue: QueueType,
        pipeline: ComputePipelineHandle,
    ) -> Self {
        Self {
            render_graph_builder,
            name: name.to_string(),
            color: [0.0, 1.0, 0.0, 1.0],
            queue,
            pipeline,
            resources: Vec::new(),
            dispatch: crate::render_graph::ComputeDispatch::Size([1; 3]),
            buffer_usages: Vec::new(),
            image_usages: Vec::new(),
        }
    }

    pub fn override_label_color(&mut self, color: [f32; 4]) {
        self.color = color;
    }

    pub fn dispatch_size(mut self, size: [u32; 3]) -> Self {
        self.dispatch = crate::render_graph::ComputeDispatch::Size(size);
        self
    }

    pub fn dispatch_indirect(mut self, buffer: BufferHandle, offset: usize) -> Self {
        let buffer = self.render_graph_builder.get_buffer_index(buffer);
        let offset = offset as u64;
        self.buffer_usages
            .push((buffer, BufferResourceAccess::IndirectRead));
        self.dispatch =
            crate::render_graph::ComputeDispatch::Indirect(crate::render_graph::BufferOffset {
                buffer,
                offset,
            });
        self
    }

    pub fn read_buffer(mut self, buffer: BufferHandle) -> Self {
        let buffer = self.render_graph_builder.get_buffer_index(buffer);
        self.buffer_usages
            .push((buffer, BufferResourceAccess::StorageRead));
        self.resources
            .push(crate::render_graph::ShaderResourceUsage::StorageBuffer {
                buffer,
                write: false,
            });
        self
    }

    pub fn write_buffer(mut self, buffer: BufferHandle) -> Self {
        let buffer = self.render_graph_builder.get_buffer_index(buffer);
        self.buffer_usages
            .push((buffer, BufferResourceAccess::StorageWrite));
        self.resources
            .push(crate::render_graph::ShaderResourceUsage::StorageBuffer {
                buffer,
                write: true,
            });
        self
    }

    pub fn read_storage_image(mut self, image: ImageHandle) -> Self {
        let image = self.render_graph_builder.get_image_index(image);
        self.image_usages
            .push((image, ImageResourceAccess::StorageRead));
        self.resources
            .push(crate::render_graph::ShaderResourceUsage::StorageImage {
                image,
                write: false,
            });
        self
    }

    pub fn write_storage_image(mut self, image: ImageHandle) -> Self {
        let image = self.render_graph_builder.get_image_index(image);
        self.image_usages
            .push((image, ImageResourceAccess::StorageWrite));
        self.resources
            .push(crate::render_graph::ShaderResourceUsage::StorageImage { image, write: true });
        self
    }

    pub fn read_sampled_image(mut self, image: ImageHandle) -> Self {
        let image = self.render_graph_builder.get_image_index(image);
        self.image_usages
            .push((image, ImageResourceAccess::SampledRead));
        self.resources
            .push(crate::render_graph::ShaderResourceUsage::SampledImage(
                image,
            ));
        self
    }

    pub fn read_sampler(mut self, sampler: SamplerHandle) -> Self {
        self.resources
            .push(crate::render_graph::ShaderResourceUsage::Sampler(sampler));
        self
    }

    pub fn build(self) {
        drop(self);
    }
}

impl<'a> Drop for ComputePassBuilder<'a> {
    fn drop(&mut self) {
        self.render_graph_builder.render_graph.render_passes.push(
            crate::render_graph::RenderPass {
                label_name: std::mem::take(&mut self.name),
                label_color: self.color,
                queue: self.queue,
                buffer_usages: std::mem::take(&mut self.buffer_usages),
                image_usages: std::mem::take(&mut self.image_usages),
                command: Some(crate::render_graph::RenderPassCommand::Compute {
                    pipeline: self.pipeline,
                    resources: std::mem::take(&mut self.resources),
                    dispatch: self.dispatch.clone(),
                }),
            },
        );
    }
}

#[derive(Debug)]
pub struct RasterPassBuilder<'a> {
    render_graph_builder: &'a mut RenderGraphBuilder,
    name: String,
    color: [f32; 4],

    framebuffer: crate::render_graph::Framebuffer,
    draw_commands: Vec<crate::render_graph::RasterDrawCommand>,

    buffer_usages: Vec<(BufferIndex, BufferResourceAccess)>,
    image_usages: Vec<(ImageIndex, ImageResourceAccess)>,
}

impl<'a> RasterPassBuilder<'a> {
    pub fn new(render_graph_builder: &'a mut RenderGraphBuilder, name: &str) -> Self {
        Self {
            render_graph_builder,
            name: name.to_string(),
            color: [0.0, 1.0, 0.0, 1.0],
            framebuffer: crate::render_graph::Framebuffer::default(),
            draw_commands: Vec::new(),
            buffer_usages: Vec::new(),
            image_usages: Vec::new(),
        }
    }

    pub fn override_label_color(&mut self, color: [f32; 4]) {
        self.color = color;
    }

    pub fn add_color_attachment(mut self, image: ImageHandle, clear: Option<[f32; 4]>) -> Self {
        let image = self.render_graph_builder.get_image_index(image);
        self.image_usages
            .push((image, ImageResourceAccess::AttachmentWrite));
        self.framebuffer
            .color_attachments
            .push(crate::render_graph::ColorAttachment { image, clear });
        self
    }

    pub fn add_depth_stencil_attachment(
        mut self,
        image: ImageHandle,
        clear: Option<(f32, u32)>,
    ) -> Self {
        assert!(
            self.framebuffer.depth_stencil_attachment.is_none(),
            "Can only set one depth stencil attachment per raster pass"
        );
        let image = self.render_graph_builder.get_image_index(image);
        self.image_usages
            .push((image, ImageResourceAccess::AttachmentWrite));
        self.framebuffer.depth_stencil_attachment =
            Some(crate::render_graph::DepthStencilAttachment { image, clear });
        self
    }

    pub fn build(self) {
        drop(self);
    }
}

impl<'a> Drop for RasterPassBuilder<'a> {
    fn drop(&mut self) {
        for (buffer, access) in self.buffer_usages.iter() {
            self.render_graph_builder.render_graph.buffer_resources[*buffer].last_access = *access;
        }

        self.render_graph_builder.render_graph.render_passes.push(
            crate::render_graph::RenderPass {
                label_name: std::mem::take(&mut self.name),
                label_color: self.color,
                queue: QueueType::Graphics,
                buffer_usages: std::mem::take(&mut self.buffer_usages),
                image_usages: std::mem::take(&mut self.image_usages),
                command: Some(RenderPassCommand::Raster {
                    framebuffer: std::mem::take(&mut self.framebuffer),
                    draw_commands: std::mem::take(&mut self.draw_commands),
                }),
            },
        );
    }
}

#[derive(Debug)]
pub struct DrawCommandBuilder<'a, 'b> {
    raster_pass_builder: &'b mut RasterPassBuilder<'a>,

    pipeline: RasterPipelineHandle,
    vertex_buffers: Vec<crate::render_graph::BufferOffset>,
    resources: Vec<crate::render_graph::ShaderResourceUsage>,
    dispatch: Option<crate::render_graph::DrawCommandDispatch>,
}

impl<'a, 'b> DrawCommandBuilder<'a, 'b> {
    pub fn new(
        raster_pass_builder: &'b mut RasterPassBuilder<'a>,
        pipeline: RasterPipelineHandle,
    ) -> Self {
        Self {
            raster_pass_builder,
            pipeline,
            vertex_buffers: Vec::new(),
            resources: Vec::new(),
            dispatch: None,
        }
    }

    pub fn add_vertex_buffer(mut self, buffer_offset: BufferOffset) -> Self {
        let buffer_offset = self
            .raster_pass_builder
            .render_graph_builder
            .get_buffer_offset(buffer_offset);
        self.raster_pass_builder
            .buffer_usages
            .push((buffer_offset.buffer, BufferResourceAccess::VertexRead));
        self.vertex_buffers.push(buffer_offset);
        self
    }

    pub fn read_buffer(mut self, buffer: BufferHandle) -> Self {
        let buffer = self
            .raster_pass_builder
            .render_graph_builder
            .get_buffer_index(buffer);
        self.raster_pass_builder
            .buffer_usages
            .push((buffer, BufferResourceAccess::StorageRead));
        self.resources
            .push(crate::render_graph::ShaderResourceUsage::StorageBuffer {
                buffer,
                write: false,
            });
        self
    }

    pub fn write_buffer(mut self, buffer: BufferHandle) -> Self {
        let buffer = self
            .raster_pass_builder
            .render_graph_builder
            .get_buffer_index(buffer);
        self.raster_pass_builder
            .buffer_usages
            .push((buffer, BufferResourceAccess::StorageWrite));
        self.resources
            .push(crate::render_graph::ShaderResourceUsage::StorageBuffer {
                buffer,
                write: true,
            });
        self
    }

    pub fn read_storage_image(mut self, image: ImageHandle) -> Self {
        let image = self
            .raster_pass_builder
            .render_graph_builder
            .get_image_index(image);
        self.raster_pass_builder
            .image_usages
            .push((image, ImageResourceAccess::StorageRead));
        self.resources
            .push(crate::render_graph::ShaderResourceUsage::StorageImage {
                image,
                write: false,
            });
        self
    }

    pub fn write_storage_image(mut self, image: ImageHandle) -> Self {
        let image = self
            .raster_pass_builder
            .render_graph_builder
            .get_image_index(image);
        self.raster_pass_builder
            .image_usages
            .push((image, ImageResourceAccess::StorageWrite));
        self.resources
            .push(crate::render_graph::ShaderResourceUsage::StorageImage { image, write: true });
        self
    }

    pub fn read_sampled_image(mut self, image: ImageHandle) -> Self {
        let image = self
            .raster_pass_builder
            .render_graph_builder
            .get_image_index(image);
        self.raster_pass_builder
            .image_usages
            .push((image, ImageResourceAccess::SampledRead));
        self.resources
            .push(crate::render_graph::ShaderResourceUsage::SampledImage(
                image,
            ));
        self
    }

    pub fn read_sampler(mut self, sampler: SamplerHandle) -> Self {
        self.resources
            .push(crate::render_graph::ShaderResourceUsage::Sampler(sampler));
        self
    }

    pub fn draw(mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.dispatch = Some(crate::render_graph::DrawCommandDispatch::Draw {
            vertices,
            instances,
        });
        drop(self);
    }

    pub fn draw_indexed(
        mut self,
        base_vertex: i32,
        indices: Range<u32>,
        instances: Range<u32>,
        index_buffer: BufferOffset,
        index_type: IndexType,
    ) {
        let index_buffer = self
            .raster_pass_builder
            .render_graph_builder
            .get_buffer_offset(index_buffer);
        self.raster_pass_builder
            .buffer_usages
            .push((index_buffer.buffer, BufferResourceAccess::IndexRead));

        self.dispatch = Some(crate::render_graph::DrawCommandDispatch::DrawIndexed {
            base_vertex,
            indices,
            instances,
            index_buffer,
            index_type,
        });
        drop(self);
    }

    pub fn draw_indirect(mut self, indirect_buffer: BufferOffset, draw_count: u32, stride: u32) {
        let indirect_buffer = self
            .raster_pass_builder
            .render_graph_builder
            .get_buffer_offset(indirect_buffer);
        self.raster_pass_builder
            .buffer_usages
            .push((indirect_buffer.buffer, BufferResourceAccess::IndirectRead));

        self.dispatch = Some(crate::render_graph::DrawCommandDispatch::DrawIndirect {
            indirect_buffer,
            draw_count,
            stride,
        });
        drop(self);
    }

    pub fn draw_indirect_indexed(
        mut self,
        indirect_buffer: BufferOffset,
        draw_count: u32,
        stride: u32,
        index_buffer: BufferOffset,
        index_type: IndexType,
    ) {
        let indirect_buffer = self
            .raster_pass_builder
            .render_graph_builder
            .get_buffer_offset(indirect_buffer);
        self.raster_pass_builder
            .buffer_usages
            .push((indirect_buffer.buffer, BufferResourceAccess::IndirectRead));

        let index_buffer = self
            .raster_pass_builder
            .render_graph_builder
            .get_buffer_offset(index_buffer);
        self.raster_pass_builder
            .buffer_usages
            .push((index_buffer.buffer, BufferResourceAccess::IndexRead));

        self.dispatch = Some(
            crate::render_graph::DrawCommandDispatch::DrawIndirectIndexed {
                indirect_buffer,
                draw_count,
                stride,
                index_buffer,
                index_type,
            },
        );
        drop(self);
    }
}

impl<'a, 'b> Drop for DrawCommandBuilder<'a, 'b> {
    fn drop(&mut self) {
        self.raster_pass_builder
            .draw_commands
            .push(crate::render_graph::RasterDrawCommand {
                pipeline: self.pipeline,
                vertex_buffers: std::mem::take(&mut self.vertex_buffers),
                resources: std::mem::take(&mut self.resources),
                dispatch: self
                    .dispatch
                    .take()
                    .expect("No draw command dispatch set for draw command"),
            });
    }
}

#[derive(Debug)]
pub struct BufferAccess {
    pub handle: BufferHandle,
    pub usage: BufferResourceAccess,
    //TODO: add access range
}

#[derive(Debug)]
pub struct ImageAccess {
    pub handle: ImageHandle,
    pub usage: ImageResourceAccess,
    //TODO: add access subresource range
}

#[derive(Default, Debug)]
pub struct RenderGraphBuilder {
    render_graph: RenderGraph,
    buffer_index_map: HashMap<BufferHandle, BufferIndex>,
    image_index_map: HashMap<ImageHandle, ImageIndex>,
}

impl RenderGraphBuilder {
    pub fn create_transient_buffer(&mut self, desc: BufferDescription) -> BufferHandle {
        let index = self.render_graph.buffer_resources.len() as BufferIndex;
        self.render_graph.buffer_resources.push(BufferGraphResource {
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

    pub fn build(self) -> RenderGraph {
        self.render_graph
    }

    pub fn get_buffer_index(&mut self, buffer_handle: BufferHandle) -> BufferIndex {
        match self.buffer_index_map.get(&buffer_handle) {
            Some(index) => *index,
            None => {
                if let BufferHandle::Persistent(buffer_key) = buffer_handle {
                    let index = self.render_graph.buffer_resources.len() as BufferIndex;
                    self.render_graph.buffer_resources.push(BufferGraphResource {
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
