use crate::vulkan::{Buffer, BufferDescription, Image, ImageDescription};
use std::rc::Rc;

use crate::render_graph::compiled_pass::RenderPassCompiled;
use crate::render_graph::{BufferHandle, ImageHandle};
use ash::vk;

//TODO: RayTracing Accesses
//TODO: make flags?
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

pub enum BufferResource {
    New(BufferDescription),
    Import(Rc<Buffer>),
}

//#[derive(PartialEq, Debug)]
pub enum ImageResource {
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
        RenderGraphDescription::SWAPCHAIN_ID
    }

    pub fn create_buffer(&mut self, buffer_description: BufferResource) -> BufferHandle {
        let description = self
            .description
            .as_mut()
            .expect("Render Graph already built");
        let new_id = description.buffers.len() as BufferHandle;
        description.buffers.push(buffer_description);
        new_id
    }

    pub fn create_image(&mut self, image_description: ImageResource) -> ImageHandle {
        // assert_ne!(
        //     image_description,
        //     ImageResource::Swapchain,
        //     "Cannot create image of ImageResourceDescription::Swapchain type"
        // );
        let description = self
            .description
            .as_mut()
            .expect("Render Graph already built");
        let new_id = description.images.len() as ImageHandle;
        description.images.push(image_description);
        new_id
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
            id: resource,
            access_type: access,
        });
        index
    }

    pub fn write_buffer(&mut self, resource: BufferHandle, access: BufferAccessType) -> usize {
        let description = self.description.as_mut().unwrap();
        let index = description.write_buffers.len();
        description.write_buffers.push(BufferResourceAccess {
            id: resource,
            access_type: access,
        });
        index
    }

    pub fn read_image(&mut self, resource: ImageHandle, access: ImageAccessType) -> usize {
        let description = self.description.as_mut().unwrap();
        let index = description.read_images.len();
        description.read_images.push(ImageResourceAccess {
            id: resource,
            access_type: access,
        });
        index
    }

    pub fn write_image(&mut self, resource: ImageHandle, access: ImageAccessType) -> usize {
        let description = self.description.as_mut().unwrap();
        let index = description.write_images.len();
        description.write_images.push(ImageResourceAccess {
            id: resource,
            access_type: access,
        });
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

    pub fn pipeline(&mut self, pipeline_description: PipelineDescription) {
        let description = self.description.as_mut().unwrap();
        description.pipelines.push(pipeline_description);
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
    passes: Vec<RenderPassDescription>,
    pub(crate) buffers: Vec<BufferResource>,
    pub(crate) images: Vec<ImageResource>,
}

impl RenderGraphDescription {
    const SWAPCHAIN_ID: ImageHandle = 0;

    fn new() -> Self {
        Self {
            passes: Vec::new(),
            buffers: vec![],
            images: vec![ImageResource::Swapchain],
        }
    }
}

struct BufferResourceAccess {
    id: BufferHandle,
    access_type: BufferAccessType,
}

struct ImageResourceAccess {
    id: ImageHandle,
    access_type: ImageAccessType,
}

pub enum PipelineDescription {
    Raster,
    Compute,
    RayTracing,
}

struct FramebufferDescription {
    //Render Pass's access type is known so doesn't need to be specified
    color_attachments: Vec<ImageHandle>,
    depth_attachment: Option<ImageHandle>,
}

pub struct RenderPassDescription {
    name: String,

    //Resources Description
    read_buffers: Vec<BufferResourceAccess>,
    write_buffers: Vec<BufferResourceAccess>,
    read_images: Vec<ImageResourceAccess>,
    write_images: Vec<ImageResourceAccess>,
    framebuffer: Option<FramebufferDescription>,

    //Pipeline Description
    pipelines: Vec<PipelineDescription>,

    //Render Function
    render_fn: Option<Box<RenderFn>>,
}

impl RenderPassDescription {
    fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            read_buffers: Vec::new(),
            write_buffers: Vec::new(),
            read_images: Vec::new(),
            write_images: Vec::new(),
            framebuffer: None,
            pipelines: Vec::new(),
            render_fn: None,
        }
    }
}

//Placeholder definitions, need to flesh out with vulkan primitives later
pub struct CommandBuffer {
    pub(crate) device: Rc<ash::Device>,
    pub(crate) command_buffer: vk::CommandBuffer,
}

type RenderFn = dyn FnOnce(&mut CommandBuffer, &RenderPassCompiled);
