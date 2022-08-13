mod device;
mod null;
mod render_graph_builder;

pub use device::Device as DeviceTrait;
pub use render_graph_builder::RenderGraphBuilder;

//TODO: Find best place for this
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum MemoryLocation {
    GpuOnly,
    CpuToGpu,
    GpuToCpu,
}

//TODO: define backends per platform
// Use enums backends for platforms that support more than 1 render api (i.e Windows 10+ -> VK/DX12)
pub type Device = null::NullDevice;
pub type Buffer = <null::NullDevice as DeviceTrait>::Buffer;
pub type Texture = <null::NullDevice as DeviceTrait>::Texture;
pub type Sampler = <null::NullDevice as DeviceTrait>::Sampler;

pub fn get_test_device() -> Device {
    null::NullDevice {}
}
