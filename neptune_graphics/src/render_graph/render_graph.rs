use crate::render_graph::framebuffer::RenderPassFramebuffer;
use crate::render_graph::{BufferResource, TextureResource};

pub trait CommandBufferTrait {}
pub type RasterFunction = dyn FnOnce(&mut dyn CommandBufferTrait);

pub enum BufferResourceDescription {
    New { description: usize },
    Imported { handle: usize },
}

pub enum TextureResourceDescription {
    New { description: usize },
    Imported { handle: usize },
    Swapchain {},
}

pub struct BufferResourceDeclaration {
    pub(crate) id: BufferResource,
    pub(crate) description: BufferResourceDescription,
}

pub struct TextureResourceDeclaration {
    pub(crate) id: TextureResource,
    pub(crate) description: TextureResourceDescription,
}

pub enum BufferReadAccess {}
pub enum BufferWriteAccess {}
pub enum TextureReadAccess {}
pub enum TextureWriteAccess {}

pub struct RenderGraph {
    buffer_declarations: Vec<BufferResourceDeclaration>,
    texture_declarations: Vec<TextureResourceDeclaration>,
    render_passes: Vec<RenderPass>,
}

pub struct RenderPass {
    name: String,

    framebuffer: Option<RenderPassFramebuffer>,

    buffer_read: Vec<(BufferResource, BufferReadAccess)>,
    buffer_write: Vec<(BufferResource, BufferWriteAccess)>,
    texture_read: Vec<(TextureResource, TextureReadAccess)>,
    texture_write: Vec<(TextureResource, TextureWriteAccess)>,

    function: Box<RasterFunction>,
}

pub enum RenderPassType {
    Raster {
        framebuffer: RenderPassFramebuffer,
        function: Box<RasterFunction>,
    },
    Compute {},
}

struct RenderPassBuilder {}

impl RenderPassBuilder {
    fn read_buffer(&mut self, buffer: BufferResource, flags: BufferReadAccess) {}
    fn write_buffer(&mut self, buffer: BufferResource, flags: BufferWriteAccess) {}
    fn read_texture(&mut self, texture: TextureResource, flags: TextureReadAccess) {}
    fn write_texture(&mut self, texture: TextureResource, flags: TextureWriteAccess) {}
}
