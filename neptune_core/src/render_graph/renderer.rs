use crate::render_backend::RenderDevice;
use crate::render_graph::render_graph::{BufferResource, ImageResource, RenderGraphDescription};
use crate::vulkan::{Buffer, Image};
use std::rc::Rc;

struct Renderer {
    device: RenderDevice,
}

impl Renderer {
    pub fn render(&mut self, render_graph: RenderGraphDescription) {
        self.create_resources(&render_graph);
    }

    //TODO: reuse resources
    fn create_resources(&mut self, render_graph: &RenderGraphDescription) {
        let buffers: Vec<Rc<Buffer>> = render_graph
            .buffers
            .iter()
            .map(|buffer_resource| match buffer_resource {
                BufferResource::New(buffer_description) => {
                    Rc::new(Buffer::new(&self.device, buffer_description))
                }
                BufferResource::Import(buffer) => buffer.clone(),
            })
            .collect();

        let images: Vec<Rc<Image>> = render_graph
            .images
            .iter()
            .map(|image_resource| match image_resource {
                //TODO: this better
                ImageResource::Swapchain => Rc::new(Image::null(
                    self.device.base.clone(),
                    self.device.allocator.clone(),
                )),
                ImageResource::New(image_description) => {
                    Rc::new(Image::new_2d(&self.device, image_description))
                }
                ImageResource::Import(image) => image.clone(),
            })
            .collect();
    }

    fn compile(&mut self, render_graph: &RenderGraphDescription) {}
}
