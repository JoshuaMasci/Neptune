use crate::{MemoryLocation, RenderGraphBuilder};

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Integrated,
    Discrete,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceVendor {
    AMD,
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

pub trait Device {
    type Buffer: Sync + Clone;
    type Texture: Sync + Clone;
    type Sampler: Sync + Clone;

    fn info(&self) -> DeviceInfo;

    fn create_buffer(
        &mut self,
        size: usize,
        memory_location: MemoryLocation,
    ) -> Option<Self::Buffer>;
    fn create_static_buffer(
        &mut self,
        memory_location: MemoryLocation,
        data: &[u8],
    ) -> Option<Self::Buffer>;

    fn create_texture(&mut self, memory_location: MemoryLocation) -> Option<Self::Texture>;
    fn create_static_texture(
        &mut self,
        memory_location: MemoryLocation,
        data: &[u8],
    ) -> Option<Self::Texture>;

    fn create_sampler(&mut self) -> Option<Self::Sampler>;

    fn render_frame(&mut self, build_graph_fn: impl FnOnce(&mut RenderGraphBuilder));
}
