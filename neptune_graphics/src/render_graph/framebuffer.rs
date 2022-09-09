use crate::render_graph::TextureResource;

pub enum LoadOp<T> {
    None,
    Clear(T),
}

pub struct Attachment<T: Clone> {
    pub texture: TextureResource,
    pub clear_value: LoadOp<T>,
}

impl<T: Clone> Attachment<T> {
    pub fn new(texture: TextureResource) -> Self {
        Self {
            texture,
            clear_value: LoadOp::None,
        }
    }

    pub fn new_with_clear(texture: TextureResource, clear_value: &T) -> Self {
        Self {
            texture,
            clear_value: LoadOp::Clear(clear_value.clone()),
        }
    }
}

//TODO: Input Attachments
pub struct RenderPassFramebuffer {
    pub color_attachment: Vec<Attachment<[f32; 4]>>,
    pub depth_attachment: Option<Attachment<(f32, u8)>>,
}
