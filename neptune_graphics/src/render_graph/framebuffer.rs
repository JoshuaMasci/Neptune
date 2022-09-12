use crate::texture::{TextureGraphResource, TextureResource};

pub enum LoadOp<ClearType> {
    None,
    Clear(ClearType),
}

pub struct Attachment<ClearType: Clone> {
    pub texture: TextureGraphResource,
    pub clear_value: LoadOp<ClearType>,
}

impl<ClearType: Clone> Attachment<ClearType> {
    pub fn new<TextureType: TextureResource>(texture: &TextureType) -> Self {
        Self {
            texture: texture.get_graph_resource(),
            clear_value: LoadOp::None,
        }
    }

    pub fn new_with_clear<TextureType: TextureResource>(
        texture: &TextureType,
        clear_value: &ClearType,
    ) -> Self {
        Self {
            texture: texture.get_graph_resource(),
            clear_value: LoadOp::Clear(clear_value.clone()),
        }
    }
}

#[derive(Default)]
//TODO: Input Attachments
pub struct RenderPassFramebuffer {
    pub color_attachment: Vec<Attachment<[f32; 4]>>,
    pub depth_stencil_attachment: Option<Attachment<(f32, u8)>>,
}
