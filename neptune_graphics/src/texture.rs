use crate::MemoryType;
use bitflags::bitflags;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
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
        match self {
            TextureFormat::Unknown
            | TextureFormat::D16Unorm
            | TextureFormat::D24UnormS8Uint
            | TextureFormat::D32Float
            | TextureFormat::D32FloatS8Uint => false,
            _ => true,
        }
    }

    pub fn is_depth(self) -> bool {
        match self {
            TextureFormat::D16Unorm
            | TextureFormat::D24UnormS8Uint
            | TextureFormat::D32Float
            | TextureFormat::D32FloatS8Uint => true,
            _ => false,
        }
    }
}

bitflags! {
    #[derive(PartialEq, Eq, Debug, Copy, Clone)]
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

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum TextureDimensions {
    D1(u32),
    D2(u32, u32),
    D3(u32, u32, u32),
}

impl TextureDimensions {
    pub fn expect_2d(&self) -> [u32; 2] {
        match self {
            TextureDimensions::D2(a, b) => [*a, *b],
            _ => panic!("TextureDimension not 2d"),
        }
    }
}

//TODO: Texture Mips + Mip Auto filler function
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct TextureDescription {
    pub format: TextureFormat,
    pub size: TextureDimensions,
    pub usage: TextureUsages,
    pub memory_type: MemoryType,
}
