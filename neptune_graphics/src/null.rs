use crate::device::{
    BufferUsage, DeviceInfo, DeviceTrait, DeviceType, DeviceVendor, SamplerCreateInfo,
    TextureCreateInfo,
};
use crate::render_graph_builder::RenderGraphBuilderImpl;
use std::sync::Arc;

pub struct NullDevice {}

impl DeviceTrait for NullDevice {
    type ComputeShader = Arc<NullShader>;

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

    fn create_buffer(&mut self, size: usize, usage: BufferUsage) -> Option<Self::Buffer> {
        Some(Arc::new(NullBuffer(0)))
    }

    fn create_static_buffer(&mut self, usage: BufferUsage, data: &[u8]) -> Option<Self::Buffer> {
        Some(Arc::new(NullBuffer(1)))
    }

    fn create_texture(&mut self, create_info: &TextureCreateInfo) -> Option<Self::Texture> {
        Some(Arc::new(NullTexture(0)))
    }

    fn create_static_texture(
        &mut self,
        create_info: &TextureCreateInfo,
        data: &[u8],
    ) -> Option<Self::Texture> {
        Some(Arc::new(NullTexture(1)))
    }

    fn create_sampler(&mut self, create_info: &SamplerCreateInfo) -> Option<Self::Sampler> {
        Some(Arc::new(NullSampler(0)))
    }

    fn create_compute_shader(&mut self, code: &[u32]) -> Option<Self::ComputeShader> {
        Some(Arc::new(NullShader(0)))
    }

    fn render_frame(&mut self, build_graph_fn: impl FnOnce(&mut RenderGraphBuilderImpl<Self>)) {
        let mut render_graph_builder = RenderGraphBuilderImpl::default();
        build_graph_fn(&mut render_graph_builder);
    }
}

pub struct NullBuffer(u32);
pub struct NullTexture(u32);
pub struct NullSampler(u32);
pub struct NullShader(u32);
