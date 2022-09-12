use crate::buffer::Buffer;
use crate::device::{DeviceInfo, DeviceTrait, DeviceType, DeviceVendor};
use crate::render_graph::{RenderGraph, RenderGraphBuilder};
use crate::sampler::{Sampler, SamplerCreateInfo};
use crate::shader::{ComputeShader, FragmentShader, VertexShader};
use crate::texture::{SwapchainTexture, Texture};
use crate::{BufferUsage, TextureCreateInfo};

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
        Some(Buffer::new_temp(0))
    }

    fn create_static_buffer(&mut self, usage: BufferUsage, data: &[u8]) -> Option<Buffer> {
        Some(Buffer::new_temp(0))
    }

    fn create_texture(&mut self, create_info: &TextureCreateInfo) -> Option<Texture> {
        Some(Texture::new_temp(0))
    }

    fn create_static_texture(
        &mut self,
        create_info: &TextureCreateInfo,
        data: &[u8],
    ) -> Option<Texture> {
        Some(Texture::new_temp(0))
    }

    fn create_sampler(&mut self, create_info: &SamplerCreateInfo) -> Option<Sampler> {
        Some(Sampler::new_temp(0))
    }

    fn create_vertex_shader(&mut self, code: &[u8]) -> Option<VertexShader> {
        Some(VertexShader::new_temp(0))
    }

    fn create_fragment_shader(&mut self, code: &[u8]) -> Option<FragmentShader> {
        Some(FragmentShader::new_temp(0))
    }

    fn create_compute_shader(&mut self, code: &[u8]) -> Option<ComputeShader> {
        Some(ComputeShader::new_temp(0))
    }

    fn render_frame(
        &mut self,
        build_graph_fn: impl FnOnce(&mut RenderGraphBuilder, Option<SwapchainTexture>),
    ) {
        let mut render_graph = RenderGraph::default();
        let mut render_graph_builder = RenderGraphBuilder::new(&mut render_graph);
        build_graph_fn(
            &mut render_graph_builder,
            Some(SwapchainTexture::new_temp()),
        );
    }
}

pub struct NullCommandBuffer {}
