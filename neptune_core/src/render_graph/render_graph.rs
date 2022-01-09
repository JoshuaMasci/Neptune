use crate::vulkan::{Buffer, BufferDescription, Image, ImageDescription};
use std::rc::Rc;

use crate::render_graph::compiled_pass::RenderPassCompiled;
use crate::render_graph::{BufferHandle, ImageHandle, RenderFn};
use ash::vk;

//TODO: RayTracing Accesses
//TODO: make flags?
#[derive(Copy, Clone)]
pub enum BufferAccessType {
    Nothing,

    IndexBuffer,
    VertexBuffer,

    TransferRead,
    TransferWrite,

    ShaderVertexRead,
    ShaderFragmentRead,

    ShaderComputeRead,
    ShaderComputeWrite,
}

//TODO: RayTracing Accesses
#[derive(Copy, Clone)]
pub enum ImageAccessType {
    Nothing,

    TransferRead,
    TransferWrite,

    //TODO: not sure if these needs to exist
    BlitRead,
    BLitWrite,

    ColorAttachmentRead,
    ColorAttachmentWrite,

    DepthStencilAttachmentRead,
    DepthStencilAttachmentWrite,

    ShaderComputeRead,
    ShaderComputeWrite,
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
    pub fn read_buffer(&mut self, resource: BufferHandle, access: BufferAccessType) -> usize {
        let description = self.description.as_mut().unwrap();
        let index = description.read_buffers.len();
        description.read_buffers.push(BufferResourceAccess {
            handle: resource,
            access_type: access,
        });
        index
    }

    pub fn write_buffer(&mut self, resource: BufferHandle, access: BufferAccessType) -> usize {
        let description = self.description.as_mut().unwrap();
        let index = description.write_buffers.len();
        description.write_buffers.push(BufferResourceAccess {
            handle: resource,
            access_type: access,
        });
        index
    }

    pub fn read_image(&mut self, resource: ImageHandle, access: ImageAccessType) -> usize {
        let description = self.description.as_mut().unwrap();
        let index = description.read_images.len();
        description.read_images.push(ImageResourceAccess {
            handle: resource,
            access_type: access,
        });
        index
    }

    pub fn write_image(&mut self, resource: ImageHandle, access: ImageAccessType) -> usize {
        let description = self.description.as_mut().unwrap();
        let index = description.write_images.len();
        description.write_images.push(ImageResourceAccess {
            handle: resource,
            access_type: access,
        });
        index
    }

    pub fn pipeline(&mut self, pipeline_description: PipelineDescription) -> usize {
        let description = self.description.as_mut().unwrap();
        let index = description.pipelines.len();
        description.pipelines.push(pipeline_description);
        index
    }

    pub fn raster(
        &mut self,
        color_attachments: Vec<ImageHandle>,
        depth_attachment: Option<ImageHandle>,
    ) {
        let description = self.description.as_mut().unwrap();
        description.framebuffer = Some(FramebufferDescription {
            color_attachments,
            depth_attachment,
        })
    }

    pub fn render(
        mut self,
        render: impl FnOnce(&mut CommandBuffer, &RenderPassCompiled) + 'static,
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
    //Render Pass's access type is known so doesn't need to be specified
    color_attachments: Vec<ImageHandle>,
    depth_attachment: Option<ImageHandle>,
}

pub struct RenderPassDescription {
    pub(crate) name: String,

    //Resources Description
    pub(crate) read_buffers: Vec<BufferResourceAccess>,
    pub(crate) write_buffers: Vec<BufferResourceAccess>,
    pub(crate) read_images: Vec<ImageResourceAccess>,
    pub(crate) write_images: Vec<ImageResourceAccess>,

    //Pipeline Description
    pub(crate) pipelines: Vec<PipelineDescription>,

    //Framebuffer, might be replaced in favor of VkDynamicRendering
    pub(crate) framebuffer: Option<FramebufferDescription>,

    //Render Function
    pub(crate) render_fn: Option<Box<RenderFn>>,
}

impl RenderPassDescription {
    fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            read_buffers: Vec::new(),
            write_buffers: Vec::new(),
            read_images: Vec::new(),
            write_images: Vec::new(),
            pipelines: Vec::new(),
            framebuffer: None,
            render_fn: None,
        }
    }
}

//Placeholder definitions, need to flesh out with vulkan primitives later
pub struct CommandBuffer {
    pub(crate) device: Rc<ash::Device>,
    pub(crate) command_buffer: vk::CommandBuffer,
}
