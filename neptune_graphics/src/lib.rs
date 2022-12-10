mod test;

pub fn test_api() {
    let mut test_device = TestDevice::new();

    const BufferSize: u32 = 1024;

    let some_buffer = test_device
        .create_buffer(BufferSize, "Some Buffer")
        .unwrap();

    let some_compute_pipeline = test_device
        .create_compute_pipeline(&[0, 1, 2, 3], "Some Compute Pipeline")
        .unwrap();

    {
        let mut render_graph_builder = RenderGraphBuilder::default();

        let buffer_graph_handle = render_graph_builder.import_buffer(some_buffer);
        let temp_buffer_graph_handle = render_graph_builder.new_buffer(BufferSize);

        render_graph_builder.add_compute_pass(
            some_compute_pipeline,
            &[BufferSize],
            &[
                ComputeResource::StorageBufferRead(buffer_graph_handle),
                ComputeResource::StorageBufferWrite(temp_buffer_graph_handle),
            ],
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
    fn create_buffer(&mut self, size: u32, name: &str) -> Result<Buffer>;
    fn destroy_buffer(&mut self, handle: Buffer) -> Result<()>;

    fn create_texture(&mut self, size: [u32; 2], name: &str) -> Result<Texture>;
    fn destroy_texture(&mut self, handle: Texture) -> Result<()>;

    fn create_sampler(&mut self, size: usize, name: &str) -> Result<Sampler>;
    fn destroy_sampler(&mut self, handle: Sampler) -> Result<()>;

    fn create_compute_pipeline(&mut self, code: &[u8], name: &str) -> Result<ComputePipeline>;
    fn destroy_compute_pipeline(&mut self, handle: ComputePipeline) -> Result<()>;

    fn execute_graph(&mut self, render_graph_builder: &mut RenderGraphBuilder) -> Result<()>;
}

//Render Graph
#[derive(Default)]
pub struct RenderGraphBuilder {
    buffer_resources: Vec<BufferGraphType>,
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
