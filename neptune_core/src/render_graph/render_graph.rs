use crate::vulkan::{Buffer, BufferDescription, Image, ImageDescription};
use std::rc::Rc;

use crate::render_graph::pipeline_cache::PipelineCache;
use crate::render_graph::{
    BufferHandle, ImageHandle, RenderApi, RenderFn, RenderGraphResources, RenderPassInfo,
};
use crate::transfer_queue::TransferQueue;
use ash::vk;

//TODO: RayTracing Accesses
//TODO: make flags?
#[derive(Copy, Clone, Debug)]
pub enum BufferAccessType {
    None,

    IndexBuffer,
    VertexBuffer,

    TransferRead,
    TransferWrite,

    ShaderRead,
    ShaderWrite,
}

//TODO: RayTracing Accesses
#[derive(Copy, Clone, Debug)]
pub enum ImageAccessType {
    None,

    TransferRead,
    TransferWrite,

    ColorAttachmentRead,
    ColorAttachmentWrite,

    DepthStencilAttachmentRead,
    DepthStencilAttachmentWrite,

    ShaderSampledRead,
    ShaderStorageRead,
    ShaderStorageWrite,

    Present,
}

pub enum BufferResourceDescription {
    New(BufferDescription),
    Import(Rc<Buffer>, BufferAccessType),
}

pub struct BufferResource {
    access_count: u16,
    pub(crate) description: BufferResourceDescription,
}

//#[derive(PartialEq, Debug)]
pub enum ImageResourceDescription {
    New(ImageDescription),
    Import(Rc<Image>, ImageAccessType),
}

pub struct ImageResource {
    access_count: u16,
    pub(crate) description: ImageResourceDescription,
}

pub struct RenderGraph {
    pub(crate) passes: Vec<RenderPass>,
    pub(crate) buffers: Vec<BufferResource>,
    pub(crate) images: Vec<ImageResource>,
}

impl RenderGraph {
    pub fn get_swapchain_handle() -> ImageHandle {
        0 //TODO: this
    }

    pub fn create_buffer(&mut self, buffer_description: BufferDescription) -> BufferHandle {
        let new_handle = self.buffers.len() as BufferHandle;
        self.buffers.push(BufferResource {
            access_count: 0,
            description: BufferResourceDescription::New(buffer_description),
        });
        new_handle
    }

    pub fn import_buffer(
        &mut self,
        buffer: Rc<Buffer>,
        last_access: BufferAccessType,
    ) -> BufferHandle {
        let new_handle = self.buffers.len() as BufferHandle;
        self.buffers.push(BufferResource {
            access_count: 0,
            description: BufferResourceDescription::Import(buffer, last_access),
        });
        new_handle
    }

    pub fn get_buffer_description(&mut self, handle: BufferHandle) -> BufferDescription {
        match &self.buffers[handle as usize].description {
            BufferResourceDescription::New(description) => *description,
            BufferResourceDescription::Import(buffer, _) => buffer.description,
        }
    }

    pub fn create_image(&mut self, image_description: ImageDescription) -> ImageHandle {
        let new_handle = self.images.len() as ImageHandle;
        self.images.push(ImageResource {
            access_count: 0,
            description: ImageResourceDescription::New(image_description),
        });
        new_handle
    }

    pub fn import_image(&mut self, image: Rc<Image>, last_access: ImageAccessType) -> ImageHandle {
        let new_handle = self.images.len() as ImageHandle;
        self.images.push(ImageResource {
            access_count: 0,
            description: ImageResourceDescription::Import(image, last_access),
        });
        new_handle
    }

    pub fn get_image_description(&mut self, handle: ImageHandle) -> ImageDescription {
        match &self.images[handle as usize].description {
            ImageResourceDescription::New(description) => *description,
            ImageResourceDescription::Import(image, _) => image.description,
        }
    }

    pub fn add_render_pass(&mut self, mut render_pass_builder: RenderPassBuilder) {
        let render_pass = render_pass_builder.description.take().unwrap();

        for buffer_accesses in render_pass.buffers_dependencies.iter() {
            self.buffers[buffer_accesses.handle as usize].access_count += 1;
        }

        for image_accesses in render_pass.images_dependencies.iter() {
            self.images[image_accesses.handle as usize].access_count += 1;
        }

        self.passes.push(render_pass);
    }
}

pub(crate) struct BufferResourceAccess {
    pub(crate) handle: BufferHandle,
    pub(crate) access_type: BufferAccessType,
}

pub(crate) struct ImageResourceAccess {
    pub(crate) handle: ImageHandle,
    pub(crate) access_type: ImageAccessType,
}

//TODO: use AttachmentLoadOp and #[derive(Eq, PartialEq)]
pub struct FramebufferDescription {
    pub(crate) color_attachments: Vec<(ImageHandle, [f32; 4])>,
    pub(crate) depth_attachment: Option<(ImageHandle, f32)>,
}

pub struct RenderPass {
    pub(crate) name: String,
    pub(crate) buffers_dependencies: Vec<BufferResourceAccess>,
    pub(crate) images_dependencies: Vec<ImageResourceAccess>,
    pub(crate) framebuffer: Option<FramebufferDescription>,
    pub(crate) render_fn: Option<Box<RenderFn>>,
}

pub struct RenderPassBuilder {
    description: Option<RenderPass>,
}

impl RenderPassBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            description: Some(RenderPass {
                name: String::from(name),
                buffers_dependencies: vec![],
                images_dependencies: vec![],
                framebuffer: None,
                render_fn: None,
            }),
        }
    }

    pub fn buffer(mut self, resource: BufferHandle, access: BufferAccessType) -> Self {
        let description = self.description.as_mut().unwrap();
        description.buffers_dependencies.push(BufferResourceAccess {
            handle: resource,
            access_type: access,
        });
        self
    }

    pub fn image(mut self, resource: ImageHandle, access: ImageAccessType) -> Self {
        let description = self.description.as_mut().unwrap();
        description.images_dependencies.push(ImageResourceAccess {
            handle: resource,
            access_type: access,
        });
        self
    }

    pub fn raster(
        mut self,
        color_attachments: Vec<(ImageHandle, [f32; 4])>,
        depth_attachment: Option<(ImageHandle, f32)>,
    ) -> Self {
        let description = self.description.as_mut().unwrap();

        //Setup image transitions
        for color_attachment_description in color_attachments.iter() {
            description.images_dependencies.push(ImageResourceAccess {
                handle: color_attachment_description.0,
                access_type: ImageAccessType::ColorAttachmentWrite,
            });
        }
        if let Some(depth_attachment_description) = &depth_attachment {
            description.images_dependencies.push(ImageResourceAccess {
                handle: depth_attachment_description.0,
                access_type: ImageAccessType::DepthStencilAttachmentWrite,
            });
        }

        description.framebuffer = Some(FramebufferDescription {
            color_attachments,
            depth_attachment,
        });
        self
    }

    pub fn render(
        mut self,
        render: impl FnOnce(
                &mut RenderApi,
                &mut PipelineCache,
                &mut TransferQueue,
                &RenderPassInfo,
                &RenderGraphResources,
            ) + 'static,
    ) -> Self {
        let prev = self
            .description
            .as_mut()
            .unwrap()
            .render_fn
            .replace(Box::new(render));

        assert!(prev.is_none(), "Already set render function");
        self
    }
}
