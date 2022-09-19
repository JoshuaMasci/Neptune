use crate::buffer::BufferGraphResource;
use crate::handle::HandleType;
use crate::render_graph::command_buffer::RasterCommand;
use crate::render_graph::framebuffer::RenderPassFramebuffer;
use crate::sampler::SamplerHandle;
use crate::shader::{ComputeShader, ShaderHandle};
use crate::texture::TextureGraphResource;
use crate::{PipelineState, VertexElement};

pub enum BufferReadAccess {}
pub enum BufferWriteAccess {}
pub enum TextureReadAccess {}
pub enum TextureWriteAccess {}

pub enum GraphResource {
    StorageBuffer {
        buffer: BufferGraphResource,
        write: bool,
    },
    StorageTexture {
        texture: TextureGraphResource,
        write: bool,
    },
    Sampler(SamplerHandle),
    SampledTexture(TextureGraphResource),
}

#[derive(Default)]
pub struct RenderGraph {
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

    //TODO: Probably shouldn't store a command list, instead using a closure, but this is easier to track resource usage for now
    pub commands: Vec<RasterCommand>,
}

pub enum RenderPassType {
    Null,
    Raster {
        framebuffer: RenderPassFramebuffer,
        pipelines: Vec<RasterPassPipeline>,
    },
    Compute {
        shader: ShaderHandle,
        dispatch: [u32; 3],
        resources: Vec<(u32, GraphResource)>,
    },
}
