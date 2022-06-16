use crate::interface::{Buffer, DeviceInfo, GraphicsShader, Sampler, Surface, Texture};
use std::sync::Arc;

pub trait Device {
    fn get_info(&self) -> DeviceInfo;

    fn add_surface(&self, surface: Arc<Surface>) -> Option<usize>;

    fn create_graphics_shader(
        &self,
        vertex_code: &[u8],
        fragment_code: Option<&[u8]>,
    ) -> Option<Arc<GraphicsShader>>;
    fn create_buffer(&self) -> Option<Arc<Buffer>>;
    fn create_texture(&self) -> Option<Arc<Texture>>;
    fn create_sampler(&self) -> Option<Arc<Sampler>>;

    fn draw_frame(&self) -> Option<()>;
}
