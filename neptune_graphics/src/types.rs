use crate::interfaces::{Buffer, RasterPipeline, Texture};
use bitflags::bitflags;
use thiserror::Error;

//TODO: use specific error type for each action
#[derive(Error, Debug)]
pub enum Error {
    #[error("no suitable device found")]
    NoSuitableDeviceFound,

    #[error("Replace with a more suitable error")]
    TempError,
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct AppInfo<'a> {
    pub(crate) name: &'a str,
    pub(crate) variant_version: u32,
    pub(crate) major_version: u32,
    pub(crate) minor_version: u32,
    pub(crate) patch_version: u32,
}

impl<'a> AppInfo<'a> {
    pub fn new(name: &'a str, version: [u32; 4]) -> Self {
        Self {
            name,
            variant_version: version[0],
            major_version: version[1],
            minor_version: version[2],
            patch_version: version[3],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Integrated,
    Discrete,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceVendor {
    Amd,
    Arm,
    ImgTec,
    Intel,
    Nvidia,
    Qualcomm,
    Unknown(u32),
}

#[derive(Debug, Clone)]
pub struct PhysicalDeviceFeatures {
    pub async_compute: bool,
    pub async_transfer: bool,
}

#[derive(Debug, Clone)]
pub struct PhysicalDeviceExtensions {
    pub dynamic_rendering: bool,
    pub mesh_shading: bool,
    pub ray_tracing: bool,
}

#[derive(Debug, Clone)]
pub struct PhysicalDeviceMemory {
    /// The amount of local memory in bytes
    pub device_local_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct PhysicalDeviceInfo {
    pub name: String,
    pub device_type: DeviceType,
    pub vendor: DeviceVendor,
    pub driver: String,
    pub memory: PhysicalDeviceMemory,
    pub features: PhysicalDeviceFeatures,
    pub extensions: PhysicalDeviceExtensions,
}

#[derive(Debug, Clone)]
pub struct DeviceCreateInfo {
    pub frames_in_flight_count: usize,
    pub features: PhysicalDeviceFeatures,
    pub extensions: PhysicalDeviceExtensions,
}

pub type HandleType = u64;

pub type SurfaceHandle = HandleType;
pub type BufferHandle = HandleType;
pub type TextureHandle = HandleType;
pub type SamplerHandle = HandleType;
pub type ComputePipelineHandle = HandleType;
pub type RasterPipelineHandle = HandleType;
pub type SwapchainHandle = HandleType;

bitflags! {
    pub struct BufferUsage: u32 {
        const VERTEX = 1 << 0;
        const INDEX = 1 << 1;
        const UNIFORM = 1 << 2;
        const STORAGE = 1 << 3;
        const INDIRECT = 1 << 4;
        const TRANSFER_SRC = 1 << 5;
        const TRANSFER_DST = 1 << 6;
        const TRANSFER = (1 << 5) | (1 << 6);
    }
}

#[derive(Debug, Clone)]
pub struct BufferDescription {
    pub size: u64,
    pub usage: BufferUsage,
}

bitflags! {
    pub struct TextureUsage: u32 {
        const RENDER_ATTACHMENT = 1 << 0;
        const INPUT_ATTACHMENT = 1 << 1;
        const SAMPLED = 1 << 2;
        const STORAGE = 1 << 3;
        const TRANSFER_SRC = 1 << 4;
        const TRANSFER_DST = 1 << 5;
        const TRANSFER = (1 << 4) | (1 << 5);
    }
}

//TODO: Add BC formats + 10 Bit formats + etc (Use WGPU format list as ref?)
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum TextureFormat {
    //Color Formats
    //TODO: which ones of these are actually worth having?
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

    //Swapchain formats (Needed cause of nvidia)
    Bgra8Unorm,
    Bgra8Srgb,
    A2Bgr10Unorm,

    //Depth Stencil Formats
    D16Unorm,
    D24UnormS8Uint,
    D32Float,
    D32FloatS8Uint,
}

impl TextureFormat {
    pub fn is_color(self) -> bool {
        !self.is_depth_stencil()
    }

    pub fn is_depth_stencil(&self) -> bool {
        matches!(
            self,
            TextureFormat::D16Unorm
                | TextureFormat::D24UnormS8Uint
                | TextureFormat::D32Float
                | TextureFormat::D32FloatS8Uint
        )
    }
}

#[derive(Debug, Clone)]
pub struct TextureDescription {
    pub size: [u32; 2],
    pub format: TextureFormat,
    pub usage: TextureUsage,
    pub sampler: Option<()>,
}

#[derive(Default, Debug, Copy, Clone)]
pub enum AddressMode {
    #[default]
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    ClampToBorder,
}

#[derive(Default, Debug, Copy, Clone)]
pub enum FilterMode {
    #[default]
    Nearest,
    Linear,
}

#[derive(Default, Debug, Copy, Clone)]
pub enum BorderColor {
    #[default]
    TransparentBlack,
    OpaqueBlack,
    OpaqueWhite,
}

#[derive(Default, Debug, Clone)]
pub struct SamplerDescription {
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
    pub mag_filter: FilterMode,
    pub min_filter: FilterMode,
    pub mip_filter: FilterMode,
    pub lod_clamp_range: Option<std::ops::Range<f32>>,
    pub anisotropy_clamp: Option<f32>,
    pub border_color: BorderColor,
    pub unnormalized_coordinates: bool,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum ShaderCode<'a> {
    Spirv(&'a [u32]),
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct ComputePipelineDescription<'a> {
    pub shader: ShaderCode<'a>,
}

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
pub enum IndexFormat {
    U16,
    U32,
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
    pub shader: ShaderCode<'a>,
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
    pub src_factor: BlendFactor,
    pub dst_factor: BlendFactor,
    pub blend_op: BlendOperation,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct BlendState {
    pub color: BlendComponent,
    pub alpha: BlendComponent,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct ColorTargetState {
    pub format: TextureFormat,
    pub blend: Option<BlendState>,
    pub write_mask: (), //TODO: color writes
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct FragmentState<'a> {
    pub shader: ShaderCode<'a>,
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
    pub front_face: FrontFace,
    pub cull_mode: Option<CullMode>,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct RasterPipelineDescription<'a> {
    pub vertex: VertexState<'a>,
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub fragment: Option<FragmentState<'a>>,
}

#[derive(Default, PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum ColorSpace {
    #[default]
    SrgbNonlinear,
}

#[derive(Default, PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum PresentMode {
    #[default]
    Fifo,
    FifoRelaxed,
    Immediate,
    Mailbox,
}

#[derive(Default, PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum CompositeAlphaMode {
    #[default]
    Opaque,
    PreMultiplied,
    PostMultiplied,
    Inherit,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct SwapchainDescription {
    pub surface_format: SurfaceFormat,
    pub present_mode: PresentMode,
    pub usage: TextureUsage,
    pub composite_alpha: CompositeAlphaMode,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct SurfaceFormat {
    pub format: TextureFormat,
    pub color_space: ColorSpace,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct SwapchainSupportInfo {
    pub surface_formats: Vec<SurfaceFormat>,
    pub present_modes: Vec<PresentMode>,
    pub usages: TextureUsage,
    pub composite_alpha_modes: Vec<CompositeAlphaMode>,
}

#[derive(Debug, Copy, Clone, Hash)]
pub enum Queue {
    Primary,
    PreferAsyncCompute,
    PreferAsyncTransfer,
}

pub enum ShaderResourceAccess<'a> {
    BufferUniformRead(&'a Buffer),
    BufferStorageRead(&'a Buffer),
    BufferStorageWrite(&'a Buffer),
    TextureSampleRead(&'a Texture),
    TextureStorageRead(&'a Texture),
    TextureStorageWrite(&'a Texture),
}

pub struct TextureCopyBuffer<'a> {
    pub buffer: &'a Buffer,
    pub offset: u64,
    pub row_length: Option<u32>,
    pub row_height: Option<u32>,
}

pub struct TextureCopyTexture<'a> {
    pub texture: &'a Texture,
    pub offset: [u32; 2],
}

pub enum Transfer<'a> {
    CopyCpuToBuffer {
        src: &'a [u8],
        dst: &'a Buffer,
        dst_offset: u64,
        copy_size: u64,
    },
    CopyCpuToTexture {
        src: &'a [u8],
        row_length: Option<u32>,
        row_height: Option<u32>,
        dst: TextureCopyTexture<'a>,
        copy_size: [u32; 2],
    },
    CopyBufferToBuffer {
        src: &'a Buffer,
        src_offset: u64,
        dst: &'a Buffer,
        dst_offset: u64,
        copy_size: u64,
    },
    CopyBufferToTexture {
        src: TextureCopyBuffer<'a>,
        dst: TextureCopyTexture<'a>,
        copy_size: [u32; 2],
    },
    CopyTextureToBuffer {
        src: TextureCopyTexture<'a>,
        dst: TextureCopyBuffer<'a>,
        copy_size: [u32; 2],
    },
    CopyTextureToTexture {
        src: TextureCopyTexture<'a>,
        dst: TextureCopyTexture<'a>,
        copy_size: [u32; 2],
    },
}

pub enum ComputeDispatch<'a> {
    Size([u32; 3]),
    Indirect { buffer: &'a Buffer, offset: u64 },
}

pub struct ColorAttachment<'a> {
    pub texture: &'a Texture,
    pub clear: Option<[f32; 4]>,
}

impl<'a> ColorAttachment<'a> {
    pub fn new(texture: &'a Texture) -> Self {
        Self {
            texture,
            clear: None,
        }
    }

    pub fn new_clear(texture: &'a Texture, clear: [f32; 4]) -> Self {
        Self {
            texture,
            clear: Some(clear),
        }
    }
}

pub struct DepthStencilAttachment<'a> {
    pub texture: &'a Texture,
    pub clear: Option<(f32, u32)>,
}

impl<'a> DepthStencilAttachment<'a> {
    pub fn new(texture: &'a Texture) -> Self {
        Self {
            texture,
            clear: None,
        }
    }

    pub fn new_clear(texture: &'a Texture, clear: (f32, u32)) -> Self {
        Self {
            texture,
            clear: Some(clear),
        }
    }
}

pub struct RasterPassDescription<'a> {
    pub color_attachments: &'a [ColorAttachment<'a>],
    pub depth_stencil_attachment: Option<DepthStencilAttachment<'a>>,
    pub input_attachments: &'a [&'a Texture],
}

//TODO: Indirect draw calls
pub enum RasterCommand<'a> {
    BindVertexBuffers {
        buffers: &'a [&'a Buffer],
    },
    BindIndexBuffer {
        buffer: &'a Buffer,
        format: IndexFormat,
    },
    BindShaderResource {
        resources: &'a [ShaderResourceAccess<'a>],
    },
    BindRasterPipeline {
        pipeline: &'a RasterPipeline,
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
        vertex_range: std::ops::Range<u32>,
        instance_range: std::ops::Range<u32>,
    },
    DrawIndexed {
        index_range: std::ops::Range<u32>,
        base_vertex: i32,
        instance_range: std::ops::Range<u32>,
    },
}
