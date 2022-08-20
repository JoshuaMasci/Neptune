mod device;
mod null;
mod pipeline;
mod render_graph_builder;

pub use device::BufferUsage;
pub use device::DeviceTrait;

pub use pipeline::*;

//TODO: define backends per platform
// Use enums backends for platforms that support more than 1 render api (i.e Windows 10+ -> VK/DX12)
pub type Device = null::NullDevice;

pub type ComputeShader = <null::NullDevice as DeviceTrait>::ComputeShader;

pub type Buffer = <null::NullDevice as DeviceTrait>::Buffer;
pub type Texture = <null::NullDevice as DeviceTrait>::Texture;
pub type Sampler = <null::NullDevice as DeviceTrait>::Sampler;

pub type RenderGraphBuilder = render_graph_builder::RenderGraphBuilderImpl<Device>;

pub fn get_test_device() -> Device {
    null::NullDevice {}
}
