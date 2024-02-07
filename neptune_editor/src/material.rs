use glam::{Vec2, Vec3, Vec4};
use neptune_vulkan::{ImageHandle, SamplerHandle};

#[derive(Debug, Clone)]
pub struct MaterialTexture {
    pub image: ImageHandle,
    pub sampler: SamplerHandle,
    pub uv_index: u32,
}

#[derive(Debug, Clone)]
pub struct Material {
    pub name: String,
    pub alpha_blending: bool,

    pub base_color: Vec4,
    pub metallic_roughness_factor: Vec2,
    pub emissive_color: Vec3,

    pub base_color_texture: Option<MaterialTexture>,
    pub metallic_roughness_texture: Option<MaterialTexture>,
    pub normal_texture: Option<MaterialTexture>,
    pub occlusion_texture: Option<(MaterialTexture, f32)>,
    pub emissive_texture: Option<MaterialTexture>,
}
