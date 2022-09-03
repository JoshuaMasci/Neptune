use crate::device::{
    Buffer, BufferUsage, DeviceInfo, DeviceTrait, DeviceType, DeviceVendor, Sampler,
    SamplerCreateInfo, Texture, TextureCreateInfo,
};
use crate::handle::Handle;

pub struct NullDevice {}

impl DeviceTrait for NullDevice {
    fn info(&self) -> DeviceInfo {
        DeviceInfo {
            name: String::from("NullDevice"),
            vendor: DeviceVendor::Unknown(0),
            device_type: DeviceType::Discrete,
        }
    }

    fn create_buffer(&mut self, size: usize, usage: BufferUsage) -> Option<Buffer> {
        Some(Buffer(Handle::new_temp(0)))
    }

    fn create_static_buffer(&mut self, usage: BufferUsage, data: &[u8]) -> Option<Buffer> {
        Some(Buffer(Handle::new_temp(0)))
    }

    fn create_texture(&mut self, create_info: &TextureCreateInfo) -> Option<Texture> {
        Some(Texture(Handle::new_temp(0)))
    }

    fn create_static_texture(
        &mut self,
        create_info: &TextureCreateInfo,
        data: &[u8],
    ) -> Option<Texture> {
        Some(Texture(Handle::new_temp(0)))
    }

    fn create_sampler(&mut self, create_info: &SamplerCreateInfo) -> Option<Sampler> {
        Some(Sampler(Handle::new_temp(0)))
    }

    // fn render_frame(&mut self, build_graph_fn: impl FnOnce(&mut RenderGraphBuilderImpl<Self>)) {
    //     let mut render_graph_builder = RenderGraphBuilderImpl::default();
    //     build_graph_fn(&mut render_graph_builder);
    // }
}
