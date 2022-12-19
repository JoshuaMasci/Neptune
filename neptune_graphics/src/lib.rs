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

    let some_raster_pipeline = test_device
        .create_raster_pipeline(
            &mut RasterPipelineDescription {
                vertex: VertexState {
                    shader: &[0, 1, 2, 3],
                    layouts: &[],
                },
                primitive: PrimitiveState {
                    front_face: FrontFace::CounterClockwise,
                    cull_mode: None,
                },
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::D16Unorm,
                    write_depth: true,
                    depth_op: CompareOperation::Less,
                }),
                fragment: Some(FragmentState {
                    shader: &[0, 1, 2, 3],
                    targets: &[ColorTargetState {
                        format: TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: (),
                    }],
                }),
            },
            "Some Raster Pipeline",
        )
        .unwrap();

    {
        let mut render_graph_builder = RenderGraphBuilder::default();

        let temp_buffer_graph_handle = render_graph_builder.new_buffer(BUFFER_SIZE);

        render_graph_builder.compute_pass(
            QueueType::Primary,
            some_compute_pipeline,
            &[BUFFER_SIZE],
            &[
                ShaderResource::StorageBufferRead(some_buffer),
                ShaderResource::StorageBufferWrite(temp_buffer_graph_handle),
            ],
        );

        let swapchain_image = render_graph_builder.swapchain_texture(swapchain);
        let temp_depth_image =
            render_graph_builder.new_texture(TextureSize::Relative(swapchain_image, [1.0; 2]));

        render_graph_builder.raster_pass(
            &[ColorAttachment::new_clear(swapchain_image, [0.0; 4])],
            Some(DepthStencilAttachment::new_clear(
                temp_depth_image,
                (1.0, 0),
            )),
            &[],
            &[],
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

use crate::test::TestDevice;
use std::ops::Range;
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
pub enum QueueType {
    /// The Primary queue
    Primary,

    /// Use the async compute queue if it exists, otherwise it will default to the Primary queue
    PreferAsyncCompute,

    /// Executes on an async transfer queue if it exists, otherwise it will use Primary
    PreferAsyncTransfer,
}

//Type Handles
pub type HandleType = u16;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Buffer {
    Handle(HandleType),
    Transient(HandleType),
}

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

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct RasterPipeline(HandleType);

//Traits
pub trait Device {
    //TODO: Get Texture Format support
    //fn get_format_support(format: TextureFormat) -> Option<()>;

    //TODO: Buffer Settings + Data Upload
    fn create_buffer(&mut self, size: u32, name: &str) -> Result<Buffer>;
    fn destroy_buffer(&mut self, handle: Buffer) -> Result<()>;

    //TODO: Texture Settings + Data Upload
    fn create_texture(&mut self, size: [u32; 2], name: &str) -> Result<Texture>;
    fn destroy_texture(&mut self, handle: Texture) -> Result<()>;

    //TODO: Sampler Settings
    fn create_sampler(&mut self, size: usize, name: &str) -> Result<Sampler>;
    fn destroy_sampler(&mut self, handle: Sampler) -> Result<()>;

    //TODO: Shader Module
    fn create_compute_pipeline(&mut self, shader: &[u8], name: &str) -> Result<ComputePipeline>;
    fn destroy_compute_pipeline(&mut self, handle: ComputePipeline) -> Result<()>;

    fn create_raster_pipeline(
        &mut self,
        raster_pipeline_description: &mut RasterPipelineDescription,
        name: &str,
    ) -> Result<RasterPipeline>;
    fn destroy_raster_pipeline(&mut self, handle: RasterPipeline) -> Result<()>;

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
    pub fn new_buffer(&mut self, size: u32) -> Buffer {
        self.buffer_resources.push(BufferGraphType::Transient(size));
        Buffer::Transient(self.buffer_resources.len() as HandleType - 1)
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

    pub fn upload_buffer(
        &mut self,
        dst_buffer: GraphResource<Buffer>,
        dst_offset: u32,
        data: &[u8],
    ) {
        let _ = dst_buffer;
        let _ = dst_offset;
        let _ = data;
    }

    pub fn upload_texture(&mut self, dst_texture: GraphResource<Texture>, data: &[u8]) {
        let _ = dst_texture;
        let _ = data;
    }

    //TODO: Should download use a callback or return a (non-async) future?
    pub fn download_buffer(
        &mut self,
        src_buffer: GraphResource<Buffer>,
        byte_range: std::ops::Range<u32>,
    ) {
    }
    pub fn download_texture(&mut self, src_texture: GraphResource<Texture>) {}

    pub fn copy_buffer_to_buffer(&mut self) {}
    pub fn copy_buffer_to_texture(&mut self) {}
    pub fn copy_texture_to_buffer(&mut self) {}
    pub fn copy_texture_to_texture(&mut self) {}
    pub fn blit_texture_to_texture(&mut self) {}

    pub fn compute_pass(
        &mut self,
        queue_type: QueueType,
        pipeline: ComputePipeline,
        dispatch_size: &[u32],
        resources: &[ShaderResource],
    ) {
        let _ = queue_type;
        let _ = pipeline;
        let _ = dispatch_size;
        let _ = resources;
    }

    pub fn raster_pass(
        &mut self,
        color_attachments: &[ColorAttachment],
        depth_stencil_attachment: Option<DepthStencilAttachment>,
        input_attachments: &[GraphResource<Texture>],
        commands: &[RasterCommands],
    ) {
        let _ = color_attachments;
        let _ = depth_stencil_attachment;
        let _ = input_attachments;
        let _ = commands;
    }
}

pub enum ShaderResource {
    UniformBufferRead(Buffer),
    StorageBufferRead(Buffer),
    StorageBufferWrite(Buffer),
    SampledTextureRead(GraphResource<Texture>),
    StorageTextureRead(GraphResource<Texture>),
    StorageTextureWrite(GraphResource<Texture>),
    SamplerRead(GraphResource<Sampler>),
}

pub enum IndexSize {
    Int16,
    Int32,
}

pub enum RasterCommands {
    BindVertexBuffers {
        buffers: Vec<Buffer>, //TODO: offsets
    },
    BindIndexBuffer {
        buffer: Buffer,
        size: IndexSize,
    },
    BindShaderResource {
        resources: Vec<ShaderResource>,
    },
    BindRasterPipeline {
        pipeline: RasterPipeline,
    },
    SetScissor {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    SetViewport {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    },
    Draw {
        vertex_range: Range<u32>,
        instance_range: Range<u32>,
    },
    DrawIndexed {
        index_range: Range<u32>,
        base_vertex: i32,
        instance_range: Range<u32>,
    },
    //TODO: Indirect draw
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

//TODO: something with this?
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

//Texture
use bitflags::bitflags;

//TODO: Add BC formats + 10 Bit formats + etc (Use WGPU format list as ref?)
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum TextureFormat {
    //Color Formats
    Unknown,
    R8Unorm,
    Rg8Unorm,
    Rgb8Unorm,
    Rgba8Unorm,

    R8Snorm,
    Rg8Snorm,
    Rgb8Snorm,
    Rgba8Snorm,

    R8Uint,
    Rg8Uint,
    Rgb8Uint,
    Rgba8Uint,

    R8Sint,
    Rg8Sint,
    Rgb8Sint,
    Rgba8Sint,

    R16Unorm,
    Rg16Unorm,
    Rgb16Unorm,
    Rgba16Unorm,

    R16Snorm,
    Rg16Snorm,
    Rgb16Snorm,
    Rgba16Snorm,

    R16Uint,
    Rg16Uint,
    Rgb16Uint,
    Rgba16Uint,

    R16Sint,
    Rg16Sint,
    Rgb16Sint,
    Rgba16Sint,

    //Depth Stencil Formats
    D16Unorm,
    D24UnormS8Uint,
    D32Float,
    D32FloatS8Uint,
}

impl TextureFormat {
    pub fn is_color(self) -> bool {
        !matches!(
            self,
            TextureFormat::Unknown
                | TextureFormat::D16Unorm
                | TextureFormat::D24UnormS8Uint
                | TextureFormat::D32Float
                | TextureFormat::D32FloatS8Uint
        )
    }

    pub fn is_depth(self) -> bool {
        matches!(
            self,
            TextureFormat::D16Unorm
                | TextureFormat::D24UnormS8Uint
                | TextureFormat::D32Float
                | TextureFormat::D32FloatS8Uint
        )
    }
}

bitflags! {
    pub struct TextureUsages: u32 {
        const TRANSFER_SRC = 1 << 0;
        const TRANSFER_DST = 1 << 1;
        const STORAGE = 1 << 2;
        const SAMPLED = 1 << 3;
        const COLOR_ATTACHMENT = 1 << 4;
        const DEPTH_STENCIL_ATTACHMENT = 1 << 5;
        const INPUT_ATTACHMENT = 1 << 6;
        const TRANSIENT_ATTACHMENT = 1 << 7;
    }
}

//Raster Pipeline State

//TODO: Add complete list from WGPU?
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum VertexFormat {
    Byte,
    Byte2,
    Byte3,
    Byte4,
    Float,
    Float2,
    Float3,
    Float4,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum VertexStepMode {
    Vertex,
    Instance,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct VertexAttribute {
    pub format: VertexFormat,
    pub offset: u32,
    pub shader_location: u32,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct VertexBufferLayout<'a> {
    pub stride: u32,
    pub step: VertexStepMode,
    pub attributes: &'a [VertexAttribute],
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct VertexState<'a> {
    pub shader: &'a [u8], //TODO: Shader Module
    pub layouts: &'a [VertexBufferLayout<'a>],
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum BlendFactor {
    Zero,
    One,
    ColorSrc,
    OneMinusColorSrc,
    ColorDst,
    OneMinusColorDst,
    AlphaSrc,
    OneMinusAlphaSrc,
    AlphaDst,
    OneMinusAlphaDst,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum BlendOperation {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct BlendComponent {
    src_factor: BlendFactor,
    dst_factor: BlendFactor,
    blend_op: BlendOperation,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct BlendState {
    color: BlendComponent,
    alpha: BlendComponent,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct ColorTargetState {
    pub format: TextureFormat,
    pub blend: Option<BlendState>,
    pub write_mask: (), //TODO: color writes
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct FragmentState<'a> {
    pub shader: &'a [u8], //TODO: Shader Module
    pub targets: &'a [ColorTargetState],
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum CompareOperation {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct DepthStencilState {
    pub format: TextureFormat,
    pub write_depth: bool,
    pub depth_op: CompareOperation,
    //TODO: Stencil State and Depth Bias
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum FrontFace {
    CounterClockwise,
    Clockwise,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum CullMode {
    Front,
    Back,
    All,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct PrimitiveState {
    front_face: FrontFace,
    cull_mode: Option<CullMode>,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct RasterPipelineDescription<'a> {
    pub vertex: VertexState<'a>,
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub fragment: Option<FragmentState<'a>>,
}
