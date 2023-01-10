use crate::resource_manager::{BufferHandle, ComputePipelineHandle, SamplerHandle, TextureHandle};
use crate::{BufferUsage, Swapchain, SwapchainHandle, TextureUsage};
use ash::vk;
use bitflags::bitflags;
use std::ops::Range;

#[derive(Debug, Clone, Hash)]
pub enum Queue {
    Primary,
    AsyncCompute,
    AsyncTransfer,
}

#[derive(Debug, Clone, Copy, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum BufferGraphResource {
    Transient(usize),
    Import(BufferHandle),
}

#[derive(Debug, Clone, Copy, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum TextureGraphResource {
    Transient(usize),
    Import(TextureHandle),
    Swapchain(SwapchainHandle),
}

bitflags! {
    pub struct BufferAccess: u32 {
        const INDEX_BUFFER_READ = 1 << 0;
        const VERTEX_BUFFER_READ = 1 << 1;
        const TRANSFER_READ = 1 << 2;
        const TRANSFER_WRITE = 1 << 3;
        const UNIFORM_READ = 1 << 4;
        const STORAGE_READ = 1 << 5;
        const STORAGE_WRITE = 1 << 6;
    }
}

bitflags! {
    pub struct TextureAccess: u32 {
        const  ATTACHMENT_WRITE  = 1 << 0;
        const  TRANSFER_READ  = 1 << 1;
        const  TRANSFER_WRITE  = 1 << 2;
        const  SAMPLED_READ  = 1 << 3;
        const  STORAGE_READ  = 1 << 4;
        const  STORAGE_WRITE  = 1 << 5;
    }
}

pub struct BufferDescription {
    name: String,
    usage: BufferUsage,
    size: u64,
}

pub enum TextureSize {
    Absolute([u32; 2]),
    Relative(TextureGraphResource, [f32; 2]),
}

pub struct TextureDescription {
    name: String,
    usage: TextureUsage,
    format: vk::Format,
    size: TextureSize,
    sampler: Option<SamplerHandle>,
}

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum ShaderResourceAccess {
    BufferUniformRead(BufferGraphResource),
    BufferStorageRead(BufferGraphResource),
    BufferStorageWrite(BufferGraphResource),
    TextureSampleRead(TextureGraphResource),
    TextureStorageRead(TextureGraphResource),
    TextureStorageWrite(TextureGraphResource),
}

pub struct ColorAttachment {
    texture: TextureGraphResource,
    clear: Option<[f32; 4]>,
}

impl ColorAttachment {
    pub fn new(texture: TextureGraphResource) -> Self {
        Self {
            texture,
            clear: None,
        }
    }

    pub fn new_clear(texture: TextureGraphResource, clear: [f32; 4]) -> Self {
        Self {
            texture,
            clear: Some(clear),
        }
    }
}

pub struct DepthStencilAttachment {
    texture: TextureGraphResource,
    clear: Option<(f32, u32)>,
}

impl DepthStencilAttachment {
    pub fn new(texture: TextureGraphResource) -> Self {
        Self {
            texture,
            clear: None,
        }
    }

    pub fn new_clear(texture: TextureGraphResource, clear: (f32, u32)) -> Self {
        Self {
            texture,
            clear: Some(clear),
        }
    }
}

pub struct VertexBuffer {
    buffer: BufferGraphResource,
    offset: u32,
}

pub enum IndexSize {
    Int16,
    Int32,
}

pub enum RasterCommand {
    BindVertexBuffers {
        buffers: Vec<VertexBuffer>,
    },
    BindIndexBuffer {
        buffer: BufferGraphResource,
        size: IndexSize,
    },
    BindShaderResource {
        resources: Vec<ShaderResourceAccess>,
    },
    BindRasterPipeline {
        pipeline: (),
    },
    SetScissor {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    SetViewport {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    },
    Draw {
        vertex_range: Range<u32>,
        instance_range: Range<u32>,
    },
    DrawIndexed {
        index_range: Range<u32>,
        base_vertex: i32,
        instance_range: Range<u32>,
    },
}

pub enum RenderPass {
    Transfer {
        //TODO: this
    },
    Compute {
        pipeline: ComputePipelineHandle,
        dispatch: [u32; 3],
        resources: Vec<ShaderResourceAccess>,
    },
    Raster {
        input_attachments: Vec<TextureGraphResource>,
        color_attachments: Vec<ColorAttachment>,
        depth_stencil_attachment: Option<DepthStencilAttachment>,
        commands: Vec<RasterCommand>,
    },
}

pub struct RenderPassDescription {
    name: String,
    queue: Queue,
    pass: RenderPass,
}

#[derive(Default)]
pub struct RenderGraphBuilder {
    transient_buffers: Vec<BufferDescription>,
    transient_textures: Vec<TextureDescription>,
    passes: Vec<RenderPassDescription>,
}

impl RenderGraphBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn acquire_swapchain_texture(&mut self, swapchain: &Swapchain) -> TextureGraphResource {
        //TODO: this
        TextureGraphResource::Swapchain(swapchain.0.handle)
    }

    pub fn crate_texture(
        &mut self,
        name: &str,
        format: vk::Format,
        size: TextureSize,
    ) -> TextureGraphResource {
        //TODO: this
        let _ = name;
        let _ = format;
        let _ = size;
        TextureGraphResource::Transient(1)
    }

    pub fn add_raster_pass(
        &mut self,
        name: &str,
        color_attachments: &[ColorAttachment],
        depth_stencil_attachment: Option<DepthStencilAttachment>,
    ) {
        //TODO: this
        let _ = name;
        let _ = color_attachments;
        let _ = depth_stencil_attachment;
    }
}

/// A limited render graph executor that is quick to implement.
/// Planned to not have the following features
/// 1. Async queues
/// 2. Render Pass Reordering
/// 3. Optimal Pipeline Barriers
/// 4. Multiple frames in flight
/// A More complete render graph executor will be built once the api is proven in
pub(crate) struct BasicLinearRenderGraphExecutor {}

impl BasicLinearRenderGraphExecutor {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn execute_graph(&mut self, render_graph_builder: RenderGraphBuilder) {}
}
