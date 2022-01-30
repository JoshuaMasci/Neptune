use crate::vulkan::{Buffer, BufferDescription, Image, ImageDescription};
use std::rc::Rc;

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
}

pub enum BufferResourceDescription {
    New(BufferDescription),
    Import(Rc<Buffer>),
}

//#[derive(PartialEq, Debug)]
pub enum ImageResourceDescription {
    Swapchain, //Not valid in create_image function
    New(ImageDescription),
    Import(Rc<Image>),
}

pub struct RenderGraphBuilder {
    description: Option<RenderGraphDescription>,
}

//TODO: get swapchain size and format
impl RenderGraphBuilder {
    pub fn new() -> Self {
        Self {
            description: Some(RenderGraphDescription::new()),
        }
    }

    pub fn get_swapchain_image_resource(&self) -> ImageHandle {
        RenderGraphDescription::SWAPCHAIN_HANDLE
    }

    pub fn create_buffer(&mut self, buffer_description: BufferResourceDescription) -> BufferHandle {
        let description = self
            .description
            .as_mut()
            .expect("Render Graph already built");
        let new_handle = description.buffers.len() as BufferHandle;
        description.buffers.push(buffer_description);
        new_handle
    }

    pub fn create_image(&mut self, image_description: ImageResourceDescription) -> ImageHandle {
        // assert_ne!(
        //     image_description,
        //     ImageResource::Swapchain,
        //     "Cannot create image of ImageResourceDescription::Swapchain type"
        // );
        let description = self
            .description
            .as_mut()
            .expect("Render Graph already built");
        let new_handle = description.images.len() as ImageHandle;
        description.images.push(image_description);
        new_handle
    }

    pub fn create_pass(&mut self, name: &str) -> RenderPassBuilder {
        RenderPassBuilder {
            rgb: self,
            description: Some(RenderPassDescription::new(name)),
        }
    }

    pub fn build(&mut self) -> RenderGraphDescription {
        self.description.take().expect("Render Graph already built")
    }
}

pub struct RenderPassBuilder<'rgb> {
    rgb: &'rgb mut RenderGraphBuilder,
    description: Option<RenderPassDescription>,
}

impl<'s> Drop for RenderPassBuilder<'s> {
    fn drop(&mut self) {
        self.rgb
            .description
            .as_mut()
            .unwrap()
            .passes
            .push(self.description.take().unwrap());
    }
}

impl<'rg> RenderPassBuilder<'rg> {
    pub fn buffer(&mut self, resource: BufferHandle, access: BufferAccessType) {
        let description = self.description.as_mut().unwrap();
        description.buffers_dependencies.push(BufferResourceAccess {
            handle: resource,
            access_type: access,
        });
    }

    pub fn image(&mut self, resource: ImageHandle, access: ImageAccessType) {
        let description = self.description.as_mut().unwrap();
        description.images_dependencies.push(ImageResourceAccess {
            handle: resource,
            access_type: access,
        });
    }

    pub fn pipeline(&mut self, pipeline_description: PipelineDescription) -> usize {
        let description = self.description.as_mut().unwrap();
        let index = description.pipelines.len();
        description.pipelines.push(pipeline_description);
        index
    }

    //TODO: clear values
    pub fn raster(
        &mut self,
        color_attachments: Vec<(ImageHandle, [f32; 4])>,
        depth_attachment: Option<(ImageHandle, f32)>,
    ) {
        //Setup image transitions
        for color_attachment_description in color_attachments.iter() {
            self.image(
                color_attachment_description.0,
                ImageAccessType::ColorAttachmentWrite,
            );
        }

        if let Some(depth_attachment_description) = &depth_attachment {
            self.image(
                depth_attachment_description.0,
                ImageAccessType::DepthStencilAttachmentWrite,
            );
        }

        let description = self.description.as_mut().unwrap();
        description.framebuffer = Some(FramebufferDescription {
            color_attachments,
            depth_attachment,
        })
    }

    pub fn render(
        mut self,
        render: impl FnOnce(&mut RenderApi, &mut TransferQueue, &RenderPassInfo, &RenderGraphResources)
            + 'static,
    ) {
        let prev = self
            .description
            .as_mut()
            .unwrap()
            .render_fn
            .replace(Box::new(render));

        assert!(prev.is_none(), "Already set render function");
    }
}

pub struct RenderGraphDescription {
    pub(crate) passes: Vec<RenderPassDescription>,
    pub(crate) buffers: Vec<BufferResourceDescription>,
    pub(crate) images: Vec<ImageResourceDescription>,
}

impl RenderGraphDescription {
    const SWAPCHAIN_HANDLE: ImageHandle = 0;

    fn new() -> Self {
        Self {
            passes: Vec::new(),
            buffers: vec![],
            images: vec![ImageResourceDescription::Swapchain],
        }
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

pub enum PipelineDescription {
    Raster,
    Compute,
    RayTracing,
}

pub struct FramebufferDescription {
    pub(crate) color_attachments: Vec<(ImageHandle, [f32; 4])>,
    pub(crate) depth_attachment: Option<(ImageHandle, f32)>,
}

pub struct RenderPassDescription {
    pub(crate) name: String,

    //Resources Description
    pub(crate) buffers_dependencies: Vec<BufferResourceAccess>,
    pub(crate) images_dependencies: Vec<ImageResourceAccess>,

    //Pipeline Description
    pub(crate) pipelines: Vec<PipelineDescription>,

    //Framebuffer Description
    pub(crate) framebuffer: Option<FramebufferDescription>,

    //Render Function
    pub(crate) render_fn: Option<Box<RenderFn>>,
}

impl RenderPassDescription {
    fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            buffers_dependencies: Vec::new(),
            images_dependencies: Vec::new(),
            pipelines: Vec::new(),
            framebuffer: None,
            render_fn: None,
        }
    }
}
