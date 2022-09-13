use crate::buffer::{BufferGraphResource, BufferHandle};
use crate::handle::HandleType;
use crate::render_graph::framebuffer::RenderPassFramebuffer;
use crate::shader::ComputeShader;
use crate::texture::TextureGraphResource;
use crate::{IndexSize, PipelineState, VertexElement};

pub trait CommandBufferTrait {}
pub type RasterFunction = dyn FnOnce(&mut dyn CommandBufferTrait);

pub struct BufferResourceDeclaration {
    pub(crate) id: usize,
}

pub struct TextureResourceDeclaration {
    pub(crate) id: usize,
}

pub enum BufferReadAccess {}
pub enum BufferWriteAccess {}
pub enum TextureReadAccess {}
pub enum TextureWriteAccess {}

#[derive(Default)]
pub struct RenderGraph {
    pub buffer_declarations: Vec<BufferResourceDeclaration>,
    pub texture_declarations: Vec<TextureResourceDeclaration>,
    pub render_passes: Vec<RenderPass>,
}

pub struct RenderPass {
    pub name: String,

    pub buffer_read: Vec<(BufferGraphResource, BufferReadAccess)>,
    pub buffer_write: Vec<(BufferGraphResource, BufferWriteAccess)>,
    pub texture_read: Vec<(TextureGraphResource, TextureReadAccess)>,
    pub texture_write: Vec<(TextureGraphResource, TextureWriteAccess)>,
    pub pass_type: RenderPassType,
}

pub enum RasterPassCommand {
    BindVertexBuffers(Vec<(BufferHandle, u32)>),
    BindIndexBuffer(BufferHandle, u32, IndexSize),
    //TODO: PushConstants(VK) / RootConstants?(DX12)
    Draw {
        vertex_range: std::ops::Range<u32>,
        instance_range: std::ops::Range<u32>,
    },
    DrawIndexed {
        index_range: std::ops::Range<u32>,
        base_vertex: i32,
        instance_range: std::ops::Range<u32>,
    },
    //TODO: Indirect draw
}

pub struct RasterPassPipeline {
    pub vertex_shader: HandleType,
    pub fragment_shader: Option<HandleType>,
    pub pipeline_state: PipelineState,
    pub vertex_layout: Vec<VertexElement>,

    //TODO: Probably shouldn't store a command list, instead using a closure, but this is easier to track resource usage for now
    pub commands: Vec<RasterPassCommand>,
}

pub enum RenderPassType {
    Null,
    Raster {
        framebuffer: RenderPassFramebuffer,
        pipelines: Vec<RasterPassPipeline>,
    },
    Compute {
        shader: ComputeShader,
        dispatch: [u32; 3],
    },
}
