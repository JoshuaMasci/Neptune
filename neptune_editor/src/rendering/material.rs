use std::path::Path;
use std::sync::Arc;
use wgpu::Device;

pub struct Material {
    pub(crate) pipeline: wgpu::RenderPipeline,
}
