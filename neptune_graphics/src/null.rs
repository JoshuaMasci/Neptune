use crate::device::{Device, DeviceInfo, DeviceType, DeviceVendor};
use crate::{MemoryLocation, RenderGraphBuilder};
use std::sync::Arc;

pub struct NullDevice {}

impl Device for NullDevice {
    type Buffer = Arc<NullBuffer>;
    type Texture = Arc<NullTexture>;
    type Sampler = Arc<NullSampler>;

    fn info(&self) -> DeviceInfo {
        DeviceInfo {
            name: String::from("NullDevice"),
            vendor: DeviceVendor::Unknown(0),
            device_type: DeviceType::Discrete,
        }
    }

    fn create_buffer(&mut self, size: usize, memory_type: MemoryLocation) -> Option<Self::Buffer> {
        Some(Arc::new(NullBuffer(0)))
    }

    fn create_static_buffer(
        &mut self,
        memory_type: MemoryLocation,
        data: &[u8],
    ) -> Option<Self::Buffer> {
        Some(Arc::new(NullBuffer(1)))
    }

    fn create_texture(&mut self, memory_type: MemoryLocation) -> Option<Self::Texture> {
        Some(Arc::new(NullTexture(0)))
    }

    fn create_static_texture(
        &mut self,
        memory_type: MemoryLocation,
        data: &[u8],
    ) -> Option<Self::Texture> {
        Some(Arc::new(NullTexture(1)))
    }

    fn create_sampler(&mut self) -> Option<Self::Sampler> {
        Some(Arc::new(NullSampler(0)))
    }

    fn render_frame(&mut self, build_graph_fn: impl FnOnce(&mut RenderGraphBuilder)) {
        let mut render_graph_builder = RenderGraphBuilder {};
        build_graph_fn(&mut render_graph_builder);
    }
}

pub struct NullBuffer(u32);
pub struct NullTexture(u32);
pub struct NullSampler(u32);
