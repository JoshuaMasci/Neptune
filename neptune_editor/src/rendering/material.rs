use std::path::Path;
use std::sync::Arc;
use wgpu::Device;

pub(crate) struct Material {
    base_color_texture: Arc<wgpu::Texture>,

    bind_group: wgpu::BindGroup,
}

impl Material {
    pub fn create_from_image(device: &Device, file_path: &Path) -> Option<Self> {
        None
    }
}
