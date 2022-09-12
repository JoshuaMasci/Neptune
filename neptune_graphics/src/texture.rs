use bitflags::bitflags;
use std::fmt::{Debug, Formatter};

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

    //TODO: BC Images

    //Depth Stencil Formats
    D16Unorm,
    D24UnormS8Uint,
    D32Float,
    D32FloatS8Uint,
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

pub type TextureHandle = u32;

pub enum TextureGraphResource {
    External(TextureHandle),
    Transient(usize), //TODO: What should this be?
    Swapchain(u32),
}

pub trait TextureResource {
    fn get_graph_resource(&self) -> TextureGraphResource;
}

pub struct Texture {
    handle: TextureHandle,
    freed_list: std::sync::Mutex<Vec<TextureHandle>>,
}

impl Texture {
    pub fn new_temp(handle: TextureHandle) -> Self {
        Self {
            handle,
            freed_list: std::sync::Mutex::new(vec![]),
        }
    }
}

impl TextureResource for Texture {
    fn get_graph_resource(&self) -> TextureGraphResource {
        TextureGraphResource::External(self.handle)
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if let Ok(mut freed_list) = self.freed_list.lock() {
            freed_list.push(self.handle);
        }
    }
}

impl Debug for Texture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("handle", &self.handle)
            .finish()
    }
}

pub struct SwapchainTexture {
    index: u32,
    description: TextureCreateInfo,
}

impl SwapchainTexture {
    pub fn new_temp() -> Self {
        Self {
            index: 0,
            description: TextureCreateInfo {
                format: TextureFormat::Rgba8Unorm,
                size: [1920, 1080],
                usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TRANSFER_WRITE,
                mip_levels: 0,
                sample_count: 0,
            },
        }
    }
}

impl TextureResource for SwapchainTexture {
    fn get_graph_resource(&self) -> TextureGraphResource {
        TextureGraphResource::Swapchain(self.index)
    }
}

impl Debug for SwapchainTexture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwapchainTexture")
            .field("index", &self.index)
            .finish()
    }
}
