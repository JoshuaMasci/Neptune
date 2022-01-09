use std::borrow::BorrowMut;
use std::collections::HashMap;

pub type BufferHandle = u32;
pub type ImageHandle = u32;

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

pub enum BufferResourceDescription {
    New(),
    Import(),
}

#[derive(PartialEq, Debug)]
pub enum ImageResourceDescription {
    Swapchain, //Not valid for creation
    New(),
    Import(),
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

    pub fn create_buffer(&mut self, buffer_description: BufferResourceDescription) -> BufferHandle {
        let description = self
            .description
            .as_mut()
            .expect("Render Graph already built");
        let new_id = description.buffers.len() as BufferHandle;
        description.buffers.push(buffer_description);
        new_id
    }

    pub fn create_image(&mut self, image_description: ImageResourceDescription) -> ImageHandle {
        assert_ne!(
            image_description,
            ImageResourceDescription::Swapchain,
            "Cannot create image of ImageResourceDescription::Swapchain type"
        );
        let description = self
            .description
            .as_mut()
            .expect("Render Graph already built");
        let new_id = description.images2d.len() as ImageHandle;
        description.images2d.push(image_description);
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
    pub fn read_buffer(&mut self, resource: BufferHandle, access: BufferAccessType) {
        let description = self.description.as_mut().unwrap();
        description.read_buffers.push(BufferResourceAccess {
            id: resource,
            access_type: access,
        });
    }

    pub fn write_buffer(&mut self, resource: BufferHandle, access: BufferAccessType) {
        let description = self.description.as_mut().unwrap();
        description.write_buffers.push(BufferResourceAccess {
            id: resource,
            access_type: access,
        });
    }

    pub fn read_image(&mut self, resource: ImageHandle, access: ImageAccessType) {
        let description = self.description.as_mut().unwrap();
        description.read_images.push(ImageResourceAccess {
            id: resource,
            access_type: access,
        });
    }

    pub fn write_image(&mut self, resource: ImageHandle, access: ImageAccessType) {
        let description = self.description.as_mut().unwrap();
        description.write_images.push(ImageResourceAccess {
            id: resource,
            access_type: access,
        });
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

    pub fn render(mut self, render: impl FnOnce(&mut RenderInfo, &RenderPassCompiled) + 'static) {
        let prev = self
            .description
            .as_mut()
            .unwrap()
            .render_fn
            .replace(Box::new(render));

        assert!(prev.is_none());
    }
}

pub struct RenderGraphDescription {
    passes: Vec<RenderPassDescription>,
    buffers: Vec<BufferResourceDescription>,
    images2d: Vec<ImageResourceDescription>,
}

impl RenderGraphDescription {
    const SWAPCHAIN_ID: ImageHandle = 0;

    fn new() -> Self {
        Self {
            passes: Vec::new(),
            buffers: vec![],
            images2d: vec![ImageResourceDescription::Swapchain],
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
pub struct RenderInfo {
    command_buffer: (),
}
pub struct RenderPassCompiled {
    name: String,
}

type RenderFn = dyn FnOnce(&mut RenderInfo, &RenderPassCompiled);

//Design for how the render_graph system might work
pub fn build_render_graph_test() {
    let mut rgb = RenderGraphBuilder::new();

    let imgui_image = build_imgui_pass(&mut rgb);

    let swapchain_image = rgb.get_swapchain_image_resource();
    build_blit_pass(&mut rgb, imgui_image, swapchain_image);

    let render_graph = rgb.build();
}

pub fn build_imgui_pass(rgb: &mut RenderGraphBuilder) -> ImageHandle {
    let output_image = rgb.create_image(ImageResourceDescription::New()); //TODO: resource_description
    let mut imgui_pass = rgb.create_pass("ImguiPass");

    imgui_pass.raster(vec![output_image], None);
    output_image
}

pub fn build_blit_pass(
    rgb: &mut RenderGraphBuilder,
    src_image: ImageHandle,
    dst_image: ImageHandle,
) {
    let mut blit_pass = rgb.create_pass("SwapchainBlitPass");
    blit_pass.read_image(src_image, ImageAccessType::BlitRead);
    blit_pass.write_image(dst_image, ImageAccessType::BLitWrite);
}
