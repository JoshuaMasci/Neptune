use crate::handle::Handle;
use bitflags::bitflags;

pub struct Buffer(pub(crate) Handle);
pub struct Texture(pub(crate) Handle);
pub struct Sampler(pub(crate) Handle);

pub struct VertexShader(pub(crate) Handle);
pub struct FragmentShader(pub(crate) Handle);
pub struct ComputeShader(pub(crate) Handle);

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
pub struct DeviceInfo {
    pub name: String,
    pub vendor: DeviceVendor,
    pub device_type: DeviceType,
}

pub trait DeviceTrait {
    fn info(&self) -> DeviceInfo;

    fn create_buffer(&mut self, size: usize, usage: BufferUsage) -> Option<Buffer>;
    fn create_static_buffer(&mut self, usage: BufferUsage, data: &[u8]) -> Option<Buffer>;

    fn create_texture(&mut self, create_info: &TextureCreateInfo) -> Option<Texture>;
    fn create_static_texture(
        &mut self,
        create_info: &TextureCreateInfo,
        data: &[u8],
    ) -> Option<Texture>;

    fn create_sampler(&mut self, create_info: &SamplerCreateInfo) -> Option<Sampler>;

    //fn render_frame(&mut self, build_graph_fn: impl FnOnce(&mut RenderGraphBuilderImpl<Self>));
}

//Buffer API
bitflags! {
    pub struct BufferUsage: u32 {
        const TRANSFER_READ = 1 << 0;
        const TRANSFER_WRITE = 1 << 1; //TODO: delete this? Almost all buffers will require this, otherwise it can't be written to from the cpu
        const VERTEX = 1 << 2;
        const INDEX = 1 << 3;
        const UNIFORM = 1 << 4;
        const STORAGE = 1 << 5;
        const INDIRECT  = 1 << 6;
    }
}

//Texture API
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TextureFormat {
    Some,
    Other,
}

bitflags! {
    pub struct TextureUsage: u32 {
        const TRANSFER_READ = 1 << 0;
        const TRANSFER_WRITE = 1 << 1;
        const SAMPLED = 1 << 2;
        const STORAGE = 1 << 3;
        const RENDER_ATTACHMENT = 1 << 4;
    }
}

//TODO: Not sure mip_levels and sample counts can/should be allowable together
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct TextureCreateInfo {
    pub format: TextureFormat,
    pub size: [u32; 2],
    pub usage: TextureUsage,
    pub mip_levels: u32,
    pub sample_count: u32,
}

//TODO: Create API for this
#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum CubeTextureFace {
    Left,
    Right,
    Up,
    Down,
    Forward,
    Backward,
}

#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct CubeTextureCreateInfo {
    pub format: TextureFormat,
    pub size: u32,
    pub usage: TextureUsage,
    pub mip_levels: u32,
}

//Sampler API
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum AddressMode {
    Repeat,
    MirrorRepeat,
    ClampToEdge,
    ClampToBorder,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum FilterMode {
    Nearest,
    Linear,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum AnisotropicFilter {
    X1,
    X2,
    X4,
    X8,
    X16,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum BorderColor {
    TransparentBlack,
    OpaqueBlack,
    OpaqueWhite,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct SamplerCreateInfo {
    mag_filter: FilterMode,
    min_filter: FilterMode,
    mip_filter: FilterMode,
    address_mode_u: AddressMode,
    address_mode_v: AddressMode,
    address_mode_w: AddressMode,
    min_lod: f32,
    max_lod: f32,
    max_anisotropy: Option<AnisotropicFilter>,
    boarder_color: BorderColor,
}
