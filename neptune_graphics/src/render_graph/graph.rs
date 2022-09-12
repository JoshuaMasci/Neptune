use crate::buffer::BufferGraphResource;
use crate::handle::HandleType;
use crate::render_graph::framebuffer::RenderPassFramebuffer;
use crate::shader::ComputeShader;
use crate::texture::TextureGraphResource;
use crate::{PipelineState, VertexElement};

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

pub struct RasterPassPipeline {
    pub vertex_shader: HandleType,
    pub fragment_shader: Option<HandleType>,
    pub pipeline_state: PipelineState,
    pub vertex_layout: Vec<VertexElement>,
    //raster_fn: impl FnOnce(),
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
