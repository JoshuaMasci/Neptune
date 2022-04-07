use crate::MemoryType;
use bitflags::bitflags;
use std::sync::Arc;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Format {
    R8_UNORM,
    RG8_UNORM,
    RGB8_UNORM,
    RGBA8_UNORM,

    R8_SNORM,
    RG8_SNORM,
    RGB8_SNORM,
    RGBA8_SNORM,

    R8_UINT,
    RG8_UINT,
    RGB8_UINT,
    RGBA8_UINT,

    R8_SINT,
    RG8_SINT,
    RGB8_SINT,
    RGBA8_SINT,

    R16_UNORM,
    RG16_UNORM,
    RGB16_UNORM,
    RGBA16_UNORM,

    R16_SNORM,
    RG16_SNORM,
    RGB16_SNORM,
    RGBA16_SNORM,

    R16_UINT,
    RG16_UINT,
    RGB16_UINT,
    RGBA16_UINT,

    R16_SINT,
    RG16_SINT,
    RGB16_SINT,
    RGBA16_SINT,
}

bitflags! {
    pub struct TextureUsages: u32 {
        const TRANSFER_SRC = 1 << 0;
        const TRANSFER_DST = 1 << 1;
        const STORAGE = 1 << 2;
        const SAMPLED = 1 << 1;
        const ATTACHMENT = 1 << 2;
    }
}

//TODO: tie this type to DeviceImpl???
pub type TextureId = u32;
pub type TextureCubeId = u32;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct TextureDescription {
    pub name: String,
    pub format: Format,
    pub usage: TextureUsages,
    pub memory_type: MemoryType,
    pub mip_levels: u32,
}

pub struct Texture {
    device: Arc<dyn crate::internal::DeviceImpl>,
    handle: TextureId,
}

pub struct TextureCube {
    device: Arc<dyn crate::internal::DeviceImpl>,
    handle: TextureCubeId,
}
