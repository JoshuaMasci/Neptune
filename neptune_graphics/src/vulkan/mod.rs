mod buffer;
mod debug_messenger;
mod descriptor_set;
mod device;
mod graph;
mod instance;
mod pipeline_cache;
mod raster_api;
mod swapchain;
mod texture;

pub use buffer::Buffer;
pub use device::Device;
pub use graph::{Graph, Pass, PassData, PassSet};
pub use instance::Instance;
pub use pipeline_cache::FramebufferLayout;
pub use texture::Texture;

use crate::{BufferDescription, TextureDescription};
use ash::vk;

//Not sure these need to exist
pub(crate) struct BufferInfo {
    pub description: BufferDescription,
    pub handle: vk::Buffer,
    pub binding: Option<u32>,
}

pub(crate) struct TextureInfo {
    pub description: TextureDescription,
    pub handle: vk::Image,
    pub view: vk::ImageView,
    pub storage_binding: Option<u32>,
    pub sampled_binding: Option<u32>,
}
