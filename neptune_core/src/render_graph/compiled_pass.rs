use crate::render_graph::render_graph::{BufferAccessType, ImageAccessType};
use crate::render_graph::RenderFn;
use crate::vulkan::{Buffer, Image};
use ash::vk;
use std::rc::Rc;

pub struct BufferResource {
    pub buffer: Rc<Buffer>,
    pub(crate) access_type: BufferAccessType,
}

pub struct ImageResource {
    pub image: Rc<Image>,
    pub(crate) access_type: ImageAccessType,
}

pub struct RenderPassCompiled {
    pub name: String,

    //Resources Description
    pub read_buffers: Vec<BufferResource>,
    pub write_buffers: Vec<BufferResource>,
    pub read_images: Vec<ImageResource>,
    pub write_images: Vec<ImageResource>,

    //Pipeline Description
    pub pipelines: Vec<vk::Pipeline>,

    pub framebuffer: Option<()>,

    //Render Function
    pub(crate) render_fn: Option<Box<RenderFn>>,
}
