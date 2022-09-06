mod device;
mod handle;
mod null;
mod pipeline;
mod render_graph;
mod render_graph_builder;

pub use device::BufferUsage;
pub use device::DeviceTrait;

pub use pipeline::*;

//TODO: define backends per platform
// Use enums backends for platforms that support more than 1 render api (i.e Windows 10+ -> VK/DX12)
pub type Device = null::NullDevice;

pub fn get_test_device() -> Device {
    null::NullDevice {}
}
