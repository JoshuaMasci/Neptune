use crate::buffer::{Buffer, BufferUsage};
use crate::render_graph::RenderGraphBuilder;
use crate::sampler::{Sampler, SamplerCreateInfo};
use crate::shader::{ComputeShader, FragmentShader, VertexShader};
use crate::texture::{SwapchainTexture, Texture, TextureCreateInfo};

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Integrated,
    Discrete,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceVendor {
    Amd,
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
    //TODO: Add VRam amount, Other Device Properties?
}

pub trait DeviceTrait {
    fn info(&self) -> DeviceInfo;

    fn create_buffer(&mut self, size: usize, usage: BufferUsage) -> Option<Buffer>; //TODO: use Result rather than Option
    fn create_static_buffer(&mut self, usage: BufferUsage, data: &[u8]) -> Option<Buffer>;

    fn create_texture(&mut self, create_info: &TextureCreateInfo) -> Option<Texture>;
    fn create_static_texture(
        &mut self,
        create_info: &TextureCreateInfo,
        data: &[u8],
    ) -> Option<Texture>;

    fn create_sampler(&mut self, create_info: &SamplerCreateInfo) -> Option<Sampler>;

    fn create_vertex_shader(&mut self, code: &[u8]) -> Option<VertexShader>;
    fn create_fragment_shader(&mut self, code: &[u8]) -> Option<FragmentShader>;
    fn create_compute_shader(&mut self, code: &[u8]) -> Option<ComputeShader>;

    fn render_frame(
        &mut self,
        build_graph_fn: impl FnOnce(&mut RenderGraphBuilder, Option<SwapchainTexture>),
    );
}
