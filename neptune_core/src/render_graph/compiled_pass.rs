use crate::vulkan::{Buffer, Image};
use ash::vk;
use std::rc::Rc;

pub struct RenderPassCompiled {
    name: String,

    //Resources Description
    pub(crate) read_buffers: Vec<Rc<Buffer>>,
    pub(crate) write_buffers: Vec<Rc<Buffer>>,
    pub(crate) read_images: Vec<Rc<Image>>,
    pub(crate) write_images: Vec<Rc<Image>>,

    //Pipeline Description
    pub(crate) pipelines: Vec<vk::Pipeline>,
}
