use crate::buffer::Buffer;
use crate::device::{DeviceInfo, DeviceTrait, DeviceType, DeviceVendor};
use crate::instance::InstanceTrait;
use crate::render_graph::{RenderGraph, RenderGraphBuilder};
use crate::sampler::{Sampler, SamplerCreateInfo};
use crate::shader::{ComputeShader, FragmentShader, VertexShader};
use crate::surface::Surface;
use crate::texture::{SwapchainTexture, Texture};
use crate::{BufferUsage, TextureCreateInfo};

pub struct NullInstance {}

impl InstanceTrait for NullInstance {
    type DeviceImpl = NullDevice;

    fn create_surface(&mut self) -> Option<Surface> {
        Some(Surface::new_temp(0))
    }

    fn select_and_create_device(
        &mut self,
        surface: Option<&Surface>,
        score_function: impl Fn(&DeviceInfo) -> u32,
    ) -> Option<Self::DeviceImpl> {
        let _ = surface;

        let devices = [
            DeviceInfo {
                name: String::from("Dummy Unknown Gpu"),
                vendor: DeviceVendor::Unknown(256),
                device_type: DeviceType::Unknown,
            },
            DeviceInfo {
                name: String::from("Dummy Integrated Gpu"),
                vendor: DeviceVendor::Intel,
                device_type: DeviceType::Integrated,
            },
            DeviceInfo {
                name: String::from("Dummy Discrete Gpu"),
                vendor: DeviceVendor::Nvidia,
                device_type: DeviceType::Discrete,
            },
        ];

        devices
            .iter()
            .map(|device_info| (device_info, score_function(device_info)))
            .max_by(|(_, score1), (_, score2)| score1.cmp(score2))
            .map(|(device_info, _score)| NullDevice {
                device_info: device_info.clone(),
            })
    }
}

pub struct NullDevice {
    pub device_info: DeviceInfo,
}

impl DeviceTrait for NullDevice {
    fn info(&self) -> DeviceInfo {
        self.device_info.clone()
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
