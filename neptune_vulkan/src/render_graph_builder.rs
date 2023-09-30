use crate::{
    BufferHandle, ComputePipelineHandle, ImageHandle, RasterPipelineHandle, SamplerHandle,
};
use std::ops::Range;

pub enum QueueType {
    Graphics,
    PreferAsyncCompute,
    ForceAsyncCompute,
}

pub struct BufferOffset {
    pub buffer: BufferHandle,
    pub offset: usize,
}
pub enum ShaderResourceUsage {
    StorageBuffer { buffer: BufferHandle, write: bool },
    StorageImage { image: ImageHandle, write: bool },
    SampledImage(ImageHandle),
    Sampler(SamplerHandle),
}

pub enum ComputeDispatch {
    Size([u32; 3]),
    Indirect(BufferOffset),
}

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

    pub fn build(self) {}
}

pub enum IndexType {
    U16,
    U32,
}

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

pub struct RasterDrawCommand {
    pipeline: RasterPipelineHandle,
    vertex_buffers: Vec<BufferOffset>,
    index_buffer: Option<(BufferOffset, IndexType)>,
    resources: Vec<ShaderResourceUsage>,
    dispatch: RasterDispatch,
}

pub struct RasterPassBuilder {
    name: String,
    framebuffer: (),
    draw_calls: Vec<RasterDispatch>,
}
