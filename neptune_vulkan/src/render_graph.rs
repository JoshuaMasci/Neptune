use crate::{
    BufferDescription, BufferKey, ComputePipelineHandle, ImageKey, RasterPipelineHandle,
    SamplerHandle, SurfaceHandle, TransientImageDesc,
};
use std::ops::Range;

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone)]
pub enum QueueType {
    #[default]
    Graphics,
    PreferAsyncCompute,
    ForceAsyncCompute,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferResourceUsage {
    #[default]
    None,
    TransferRead,
    TransferWrite,
    VertexRead,
    IndexRead,
    IndirectRead,
    UniformRead,
    StorageRead,
    StorageWrite,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageResourceUsage {
    #[default]
    None,
    TransferRead,
    TransferWrite,
    AttachmentWrite,
    SampledRead,
    StorageRead,
    StorageWrite,
}

pub type BufferIndex = usize;

#[derive(Debug)]
pub enum BufferResourceDescription {
    Persistent(BufferKey),
    Transient(BufferDescription),
}

pub type ImageIndex = usize;

#[derive(Debug)]
pub enum ImageResourceDescription {
    Persistent(ImageKey),
    Transient(TransientImageDesc),
    Swapchain(usize),
}

#[derive(Debug, Default)]
pub struct RenderGraph {
    pub buffer_descriptions: Vec<BufferResourceDescription>,
    pub image_descriptions: Vec<ImageResourceDescription>,
    pub swapchain_images: Vec<(SurfaceHandle, ImageIndex)>,
    pub render_passes: Vec<RenderPass>,
}

#[derive(Debug)]
pub struct RenderPass {
    pub label_name: String,
    pub label_color: [f32; 4],
    pub queue: QueueType,
    pub buffer_usages: Vec<(BufferIndex, BufferResourceUsage)>,
    pub image_usages: Vec<(ImageIndex, ImageResourceUsage)>,
    pub command: Option<RenderPassCommand>,
}

#[derive(Debug)]
pub enum RenderPassCommand {
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

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BufferOffset {
    pub buffer: BufferIndex,
    pub offset: u64,
}

#[derive(Debug)]
pub enum ShaderResourceUsage {
    StorageBuffer { buffer: BufferIndex, write: bool },
    StorageImage { image: ImageIndex, write: bool },
    SampledImage(ImageIndex),
    Sampler(SamplerHandle),
}

//Transfer
#[derive(Debug, Eq, PartialEq, Copy, Clone)]

pub struct ImageCopyBuffer {
    pub buffer: BufferIndex,
    pub offset: u64,
    pub row_length: Option<u32>,
    pub row_height: Option<u32>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]

pub struct ImageCopyImage {
    pub image: ImageIndex,
    pub offset: [u32; 2],
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Transfer {
    BufferToBuffer {
        src: BufferOffset,
        dst: BufferOffset,
        copy_size: u64,
    },
    BufferToImage {
        src: ImageCopyBuffer,
        dst: ImageCopyImage,
        copy_size: [u32; 2],
    },
    ImageToBuffer {
        src: ImageCopyImage,
        dst: ImageCopyBuffer,
        copy_size: [u32; 2],
    },
    ImageToImage {
        src: ImageCopyImage,
        dst: ImageCopyImage,
        copy_size: [u32; 2],
    },
}

//Compute
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ComputeDispatch {
    Size([u32; 3]),
    Indirect(BufferOffset),
}

//Raster
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum IndexType {
    U16,
    U32,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct ColorAttachment {
    pub image: ImageIndex,
    pub clear: Option<[f32; 4]>,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct DepthStencilAttachment {
    pub image: ImageIndex,
    pub clear: Option<(f32, u32)>,
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
