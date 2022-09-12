mod buffer;
mod device;
mod handle;
mod null;
mod pipeline;
mod render_graph;
mod sampler;
mod shader;
mod texture;

pub use device::DeviceTrait;

pub use buffer::BufferUsage;

pub use texture::TextureCreateInfo;
pub use texture::TextureFormat;
pub use texture::TextureUsage;

pub use sampler::AddressMode;
pub use sampler::AnisotropicFilter;
pub use sampler::BorderColor;
pub use sampler::FilterMode;
pub use sampler::SamplerCreateInfo;

pub use pipeline::*;

pub use render_graph::Attachment;
pub use render_graph::RasterPass;

//TODO: define backends per platform
// Use enums backends for platforms that support more than 1 render api (i.e Windows 10+ -> VK/DX12)
pub type Device = null::NullDevice;
pub type CommandBuffer = null::NullCommandBuffer;

pub fn get_test_device() -> Device {
    null::NullDevice {}
}
