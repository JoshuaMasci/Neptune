mod framebuffer;
mod render_graph;

pub type ResourceId = u16;

#[derive(Copy, Clone, Default)]
pub struct BufferResource(ResourceId);

#[derive(Copy, Clone, Default)]
pub struct TextureResource(ResourceId);
