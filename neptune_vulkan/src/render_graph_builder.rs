use crate::render_graph::{CompiledRenderGraph, IndexType, QueueType};
use crate::{
    BufferDescription, BufferHandle, ComputePipelineHandle, ImageHandle, RasterPipelineHandle,
    SamplerHandle, SurfaceHandle, TransientImageDesc,
};
use ash::vk;
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

#[derive(Debug, Clone)]
pub enum ShaderResourceUsage {
    StorageBuffer { buffer: BufferHandle, write: bool },
    StorageImage { image: ImageHandle, write: bool },
    SampledImage(ImageHandle),
    Sampler(SamplerHandle),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ComputeDispatch {
    Size([u32; 3]),
    Indirect(BufferOffset),
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct ColorAttachment {
    pub image: ImageHandle,
    pub clear: Option<[f32; 4]>,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct DepthStencilAttachment {
    pub image: ImageHandle,
    pub clear: Option<(f32, u32)>,
}

#[derive(Default, Debug)]
pub struct Framebuffer {
    pub color_attachments: Vec<ColorAttachment>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum DrawCommandDispatch {
    Draw {
        vertices: Range<u32>,
        instances: Range<u32>,
    },
    DrawIndexed {
        base_vertex: i32,
        indices: Range<u32>,
        instances: Range<u32>,
        index_buffer: BufferOffset,
        index_type: IndexType,
    },
    DrawIndirect {
        indirect_buffer: BufferOffset,
        draw_count: u32,
        stride: u32,
    },
    DrawIndirectIndexed {
        indirect_buffer: BufferOffset,
        draw_count: u32,
        stride: u32,
        index_buffer: BufferOffset,
        index_type: IndexType,
    },
}

#[derive(Debug)]
pub struct RasterDrawCommand {
    pub pipeline: RasterPipelineHandle,
    pub vertex_buffers: Vec<BufferOffset>,
    pub resources: Vec<ShaderResourceUsage>,
    pub dispatch: DrawCommandDispatch,
}

// Render Graph Builder Evolution
// 0. Whole pipeline barriers between passes, no image layout changes (only general layout), no pass order changes, no dead-code culling (DONE!)
// 1. Specific pipeline barriers between passes with image layout changes, no pass order changes, no dead-code culling
// 2. Whole graph evaluation with pass reordering and dead code culling
// 3. Multi-Queue execution
pub trait RenderGraphBuilderTrait {
    fn create_transient_buffer(&mut self, desc: BufferDescription) -> BufferHandle;
    fn create_transient_image(&mut self, desc: TransientImageDesc) -> ImageHandle;
    fn acquire_swapchain_image(&mut self, surface_handle: SurfaceHandle) -> ImageHandle;

    fn add_transfer_pass(
        &mut self,
        name: String,
        color: [f32; 4],
        queue: QueueType,
        transfers: &[Transfer],
    );
    fn add_compute_pass(
        &mut self,
        name: String,
        color: [f32; 4],
        queue: QueueType,
        pipeline: ComputePipelineHandle,
        dispatch: ComputeDispatch,
        resources: &[ShaderResourceUsage],
    );
    fn add_raster_pass(
        &mut self,
        name: String,
        color: [f32; 4],
        color_attachments: &[ColorAttachment],
        depth_stencil_attachment: Option<DepthStencilAttachment>,
        raster_draw_commands: &[RasterDrawCommand],
    );

    fn build(self) -> CompiledRenderGraph;
}

//Helper Structs
pub struct TransferPassBuilder {
    name: String,
    color: [f32; 4],
    queue: QueueType,
    transfers: Vec<Transfer>,
}

impl TransferPassBuilder {
    pub fn new(name: &str, queue: QueueType) -> Self {
        Self {
            name: name.to_string(),
            color: [1.0, 0.0, 0.0, 1.0],
            queue,
            transfers: Vec::new(),
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
        self.transfers.push(Transfer::CopyBufferToBuffer {
            src,
            dst,
            copy_size: copy_size as vk::DeviceSize,
        });
    }

    pub fn copy_buffer_to_image(
        &mut self,
        src: ImageCopyBuffer,
        dst: ImageCopyImage,
        copy_size: [u32; 2],
    ) {
        self.transfers.push(Transfer::CopyBufferToImage {
            src,
            dst,
            copy_size,
        });
    }

    pub fn copy_image_to_buffer(
        &mut self,
        src: ImageCopyImage,
        dst: ImageCopyBuffer,
        copy_size: [u32; 2],
    ) {
        self.transfers.push(Transfer::CopyImageToBuffer {
            src,
            dst,
            copy_size,
        });
    }

    pub fn copy_image_to_image(
        &mut self,
        src: ImageCopyImage,
        dst: ImageCopyImage,
        copy_size: [u32; 2],
    ) {
        self.transfers.push(Transfer::CopyImageToImage {
            src,
            dst,
            copy_size,
        });
    }

    pub fn build<T: RenderGraphBuilderTrait>(self, render_graph_builder: &mut T) {
        render_graph_builder.add_transfer_pass(self.name, self.color, self.queue, &self.transfers);
    }
}

pub struct ComputePassBuilder {
    name: String,
    color: [f32; 4],
    queue: QueueType,
    pipeline: ComputePipelineHandle,
    resources: Vec<ShaderResourceUsage>,
    dispatch: ComputeDispatch,
}

impl ComputePassBuilder {
    pub fn new(name: &str, queue: QueueType, pipeline: ComputePipelineHandle) -> Self {
        Self {
            name: name.to_string(),
            color: [0.0, 1.0, 0.0, 1.0],
            queue,
            pipeline,
            resources: Vec::new(),
            dispatch: ComputeDispatch::Size([1; 3]),
        }
    }

    pub fn override_label_color(&mut self, color: [f32; 4]) {
        self.color = color;
    }

    pub fn dispatch_size(&mut self, size: [u32; 3]) {
        self.dispatch = ComputeDispatch::Size(size);
    }

    pub fn dispatch_indirect(&mut self, buffer: BufferHandle, offset: usize) {
        self.dispatch = ComputeDispatch::Indirect(BufferOffset { buffer, offset });
    }

    pub fn read_buffer(&mut self, buffer: BufferHandle) {
        self.resources.push(ShaderResourceUsage::StorageBuffer {
            buffer,
            write: false,
        });
    }

    pub fn write_buffer(&mut self, buffer: BufferHandle) {
        self.resources.push(ShaderResourceUsage::StorageBuffer {
            buffer,
            write: true,
        });
    }

    pub fn read_storage_image(&mut self, image: ImageHandle) {
        self.resources.push(ShaderResourceUsage::StorageImage {
            image,
            write: false,
        });
    }

    pub fn write_storage_image(&mut self, image: ImageHandle) {
        self.resources
            .push(ShaderResourceUsage::StorageImage { image, write: true });
    }

    pub fn read_sampled_image(&mut self, image: ImageHandle) {
        self.resources
            .push(ShaderResourceUsage::SampledImage(image));
    }

    pub fn read_sampler(&mut self, sampler: SamplerHandle) {
        self.resources.push(ShaderResourceUsage::Sampler(sampler));
    }

    pub fn build<T: RenderGraphBuilderTrait>(self, render_graph_builder: &mut T) {
        render_graph_builder.add_compute_pass(
            self.name,
            self.color,
            self.queue,
            self.pipeline,
            self.dispatch,
            &self.resources,
        );
    }
}

pub struct RasterPassBuilder {
    name: String,
    color: [f32; 4],
    framebuffer: Framebuffer,
    draw_commands: Vec<RasterDrawCommand>,
}

impl RasterPassBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            color: [0.0, 1.0, 0.0, 1.0],
            framebuffer: Framebuffer::default(),
            draw_commands: Vec::new(),
        }
    }

    pub fn override_label_color(&mut self, color: [f32; 4]) {
        self.color = color;
    }

    pub fn add_color_attachment(&mut self, image: ImageHandle, clear: Option<[f32; 4]>) {
        self.framebuffer
            .color_attachments
            .push(ColorAttachment { image, clear });
    }

    pub fn add_depth_stencil_attachment(&mut self, image: ImageHandle, clear: Option<(f32, u32)>) {
        self.framebuffer.depth_stencil_attachment = Some(DepthStencilAttachment { image, clear });
    }

    pub fn add_draw_command(&mut self, draw_command: RasterDrawCommand) {
        self.draw_commands.push(draw_command);
    }

    pub fn build<T: RenderGraphBuilderTrait>(self, render_graph_builder: &mut T) {
        render_graph_builder.add_raster_pass(
            self.name,
            self.color,
            &self.framebuffer.color_attachments,
            self.framebuffer.depth_stencil_attachment,
            &self.draw_commands,
        );
    }
}

pub struct RasterDrawCommandBuilder {
    pub pipeline: RasterPipelineHandle,
    pub vertex_buffers: Vec<BufferOffset>,
    pub resources: Vec<ShaderResourceUsage>,
    pub dispatch: Option<DrawCommandDispatch>,
}

impl RasterDrawCommandBuilder {
    pub fn new(pipeline: RasterPipelineHandle) -> Self {
        Self {
            pipeline,
            vertex_buffers: Vec::new(),
            resources: Vec::new(),
            dispatch: None,
        }
    }

    pub fn add_vertex_buffer(&mut self, buffer_offset: BufferOffset) {
        self.vertex_buffers.push(buffer_offset);
    }

    pub fn read_buffer(&mut self, buffer: BufferHandle) {
        self.resources.push(ShaderResourceUsage::StorageBuffer {
            buffer,
            write: false,
        });
    }

    pub fn write_buffer(&mut self, buffer: BufferHandle) {
        self.resources.push(ShaderResourceUsage::StorageBuffer {
            buffer,
            write: true,
        });
    }

    pub fn read_storage_image(&mut self, image: ImageHandle) {
        self.resources.push(ShaderResourceUsage::StorageImage {
            image,
            write: false,
        });
    }

    pub fn write_storage_image(&mut self, image: ImageHandle) {
        self.resources
            .push(ShaderResourceUsage::StorageImage { image, write: true });
    }

    pub fn read_sampled_image(&mut self, image: ImageHandle) {
        self.resources
            .push(ShaderResourceUsage::SampledImage(image));
    }

    pub fn read_sampler(&mut self, sampler: SamplerHandle) {
        self.resources.push(ShaderResourceUsage::Sampler(sampler));
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.dispatch = Some(DrawCommandDispatch::Draw {
            vertices,
            instances,
        });
    }

    pub fn draw_indexed(
        &mut self,
        base_vertex: i32,
        indices: Range<u32>,
        instances: Range<u32>,
        index_buffer: BufferOffset,
        index_type: IndexType,
    ) {
        self.dispatch = Some(DrawCommandDispatch::DrawIndexed {
            base_vertex,
            indices,
            instances,
            index_buffer,
            index_type,
        });
    }

    pub fn draw_indirect(&mut self, indirect_buffer: BufferOffset, draw_count: u32, stride: u32) {
        self.dispatch = Some(DrawCommandDispatch::DrawIndirect {
            indirect_buffer,
            draw_count,
            stride,
        });
    }

    pub fn draw_indirect_indexed(
        &mut self,
        indirect_buffer: BufferOffset,
        draw_count: u32,
        stride: u32,
        index_buffer: BufferOffset,
        index_type: IndexType,
    ) {
        self.dispatch = Some(DrawCommandDispatch::DrawIndirectIndexed {
            indirect_buffer,
            draw_count,
            stride,
            index_buffer,
            index_type,
        });
    }

    pub fn build(self, raster_pass_builder: &mut RasterPassBuilder) {
        raster_pass_builder.draw_commands.push(RasterDrawCommand {
            pipeline: self.pipeline,
            vertex_buffers: self.vertex_buffers,
            resources: self.resources,
            dispatch: self
                .dispatch
                .expect("No draw command dispatch set for this draw command"),
        })
    }
}
