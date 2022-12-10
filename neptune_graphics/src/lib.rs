mod test;

pub fn test_api<
    WindowType: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
>(
    window: &WindowType,
) {
    let mut test_device = TestDevice::new();
    let swapchain = test_device.create_swapchain(window).unwrap();

    const BUFFER_SIZE: u32 = 1024;

    let some_buffer = test_device
        .create_buffer(BUFFER_SIZE, "Some Buffer")
        .unwrap();

    let some_compute_pipeline = test_device
        .create_compute_pipeline(&[0, 1, 2, 3], "Some Compute Pipeline")
        .unwrap();

    {
        let mut render_graph_builder = RenderGraphBuilder::default();

        let buffer_graph_handle = render_graph_builder.import_buffer(some_buffer);
        let temp_buffer_graph_handle = render_graph_builder.new_buffer(BUFFER_SIZE);

        render_graph_builder.add_compute_pass(
            some_compute_pipeline,
            &[BUFFER_SIZE],
            &[
                ComputeResource::StorageBufferRead(buffer_graph_handle),
                ComputeResource::StorageBufferWrite(temp_buffer_graph_handle),
            ],
        );

        let swapchain_image = render_graph_builder.swapchain_texture(swapchain);
        let temp_depth_image =
            render_graph_builder.new_texture(TextureSize::Relative(swapchain_image, [1.0; 2]));

        render_graph_builder.add_raster_pass(
            &[ColorAttachment::new_clear(swapchain_image, [0.0; 4])],
            Some(DepthStencilAttachment::new_clear(
                temp_depth_image,
                (1.0, 0),
            )),
        );

        test_device
            .execute_graph(&mut render_graph_builder)
            .unwrap();
    }

    test_device
        .destroy_compute_pipeline(some_compute_pipeline)
        .unwrap();
    test_device.destroy_buffer(some_buffer).unwrap();
}

//Result and Error
use crate::test::TestDevice;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid handle")]
    InvalidHandle,

    #[error("unknown error")]
    Unknown,
}
pub type Result<T> = std::result::Result<T, Error>;

//Types
pub enum Queue {
    Graphics,
    Compute,
    Transfer,
}

//Type Handles
pub type HandleType = u16;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct Buffer(HandleType);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct Texture(HandleType);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct CubeTexture(HandleType);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct Sampler(HandleType);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct Swapchain(HandleType);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct ComputePipeline(HandleType);

//Traits
pub trait Device {
    //TODO: Buffer Settings + Data Upload
    fn create_buffer(&mut self, size: u32, name: &str) -> Result<Buffer>;
    fn destroy_buffer(&mut self, handle: Buffer) -> Result<()>;

    //TODO: Texture Settings + Data Upload
    fn create_texture(&mut self, size: [u32; 2], name: &str) -> Result<Texture>;
    fn destroy_texture(&mut self, handle: Texture) -> Result<()>;

    //TODO: Sampler Settings
    fn create_sampler(&mut self, size: usize, name: &str) -> Result<Sampler>;
    fn destroy_sampler(&mut self, handle: Sampler) -> Result<()>;

    fn create_compute_pipeline(&mut self, code: &[u8], name: &str) -> Result<ComputePipeline>;
    fn destroy_compute_pipeline(&mut self, handle: ComputePipeline) -> Result<()>;

    //TODO: Use Surface? + Swapchain Settings
    fn create_swapchain<
        WindowType: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    >(
        &mut self,
        window: &WindowType,
    ) -> Result<Swapchain>;
    fn destroy_swapchain(&mut self, handle: Swapchain) -> Result<()>;
    fn update_swapchain(&mut self, handle: Swapchain) -> Result<()>;

    fn execute_graph(&mut self, render_graph_builder: &mut RenderGraphBuilder) -> Result<()>;
}

//Render Graph
#[derive(Default)]
pub struct RenderGraphBuilder {
    buffer_resources: Vec<BufferGraphType>,
    texture_resources: Vec<TextureGraphType>,
}

impl RenderGraphBuilder {
    pub fn import_buffer(&mut self, handle: Buffer) -> GraphResource<Buffer> {
        self.buffer_resources
            .push(BufferGraphType::Imported(handle));
        GraphResource::new(self.buffer_resources.len() as HandleType - 1)
    }

    pub fn new_buffer(&mut self, size: u32) -> GraphResource<Buffer> {
        self.buffer_resources.push(BufferGraphType::Transient(size));
        GraphResource::new(self.buffer_resources.len() as HandleType - 1)
    }

    pub fn import_texture(&mut self, handle: Texture) -> GraphResource<Texture> {
        self.texture_resources
            .push(TextureGraphType::Imported(handle));
        GraphResource::new(self.texture_resources.len() as HandleType - 1)
    }

    pub fn new_texture(&mut self, size: TextureSize) -> GraphResource<Texture> {
        self.texture_resources
            .push(TextureGraphType::Transient(size));
        GraphResource::new(self.texture_resources.len() as HandleType - 1)
    }

    pub fn swapchain_texture(&mut self, handle: Swapchain) -> GraphResource<Texture> {
        self.texture_resources
            .push(TextureGraphType::Swapchain(handle));
        GraphResource::new(self.texture_resources.len() as HandleType - 1)
    }

    pub fn add_transfer_pass(&mut self) {}

    pub fn add_compute_pass(
        &mut self,
        pipeline: ComputePipeline,
        dispatch_size: &[u32],
        resources: &[ComputeResource],
    ) {
        let _ = pipeline;
        let _ = dispatch_size;
        let _ = resources;
    }

    pub fn add_raster_pass(
        &mut self,
        color_attachments: &[ColorAttachment],
        depth_stencil_attachment: Option<DepthStencilAttachment>,
    ) {
        let _ = color_attachments;
        let _ = depth_stencil_attachment;
    }
}

pub enum ComputeResource {
    UniformBufferRead(GraphResource<Buffer>),
    StorageBufferRead(GraphResource<Buffer>),
    StorageBufferWrite(GraphResource<Buffer>),
    SampledTextureRead(GraphResource<Texture>),
    StorageTextureRead(GraphResource<Texture>),
    StorageTextureWrite(GraphResource<Texture>),
    SamplerRead(GraphResource<Sampler>),
}

enum BufferGraphType {
    Imported(Buffer),
    Transient(u32),
}

enum TextureGraphType {
    Imported(Texture),
    Transient(TextureSize),
    Swapchain(Swapchain),
}

pub enum TextureSize {
    Absolute([u32; 2]),
    Relative(GraphResource<Texture>, [f32; 2]),
}

#[derive(Clone, Copy)]
pub struct GraphResource<T> {
    handle: HandleType,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> GraphResource<T> {
    fn new(handle: HandleType) -> Self {
        Self {
            handle,
            _phantom: Default::default(),
        }
    }
}

pub struct ColorAttachment {
    texture: GraphResource<Texture>,
    clear: Option<[f32; 4]>,
}

impl ColorAttachment {
    pub fn new(texture: GraphResource<Texture>) -> Self {
        Self {
            texture,
            clear: None,
        }
    }

    pub fn new_clear(texture: GraphResource<Texture>, clear: [f32; 4]) -> Self {
        Self {
            texture,
            clear: Some(clear),
        }
    }
}

pub struct DepthStencilAttachment {
    texture: GraphResource<Texture>,
    clear: Option<(f32, u32)>,
}

impl DepthStencilAttachment {
    pub fn new(texture: GraphResource<Texture>) -> Self {
        Self {
            texture,
            clear: None,
        }
    }

    pub fn new_clear(texture: GraphResource<Texture>, clear: (f32, u32)) -> Self {
        Self {
            texture,
            clear: Some(clear),
        }
    }
}

pub enum RenderPassOperations {
    //Transfer
    BufferUpload,
    TextureUpload,
    BufferDownload,
    TextureDownload,

    BufferCopy,
    TextureCopy,
    TextureBlit,

    //Compute
    Compute,

    //Render
    Raster,
}
