mod pipeline_cache;
pub mod render_graph;
mod renderer;

use crate::render_backend::RenderDevice;
use crate::render_graph::render_graph::RenderGraphDescription;
pub use crate::render_graph::renderer::Renderer;
use crate::vulkan::{Buffer, Image};

use ash::vk;

pub type BufferHandle = u32;
pub type ImageHandle = u32;

pub type RenderFn = dyn FnOnce(&mut RenderApi, &RenderPassInfo, &RenderGraphResources);

pub struct RenderApi {
    pub device: RenderDevice,
    pub command_buffer: vk::CommandBuffer,
    //pub transfer_queue: vk::CommandBuffer,
}

pub struct RenderPassInfo {
    pub name: String,
    pub pipelines: Vec<vk::Pipeline>,
    pub frame_buffer_size: Option<vk::Extent2D>,
}

pub struct RenderGraphResources {
    buffers: Vec<Buffer>,
    images: Vec<Image>,
}

//Design for how the render_graph system might work
use crate::vulkan::{BufferDescription, ImageDescription};
pub fn build_render_graph_test() -> RenderGraphDescription {
    let mut rgb = render_graph::RenderGraphBuilder::new();
    build_imgui_pass(&mut rgb);
    rgb.build()
}

pub fn build_imgui_pass(rgb: &mut render_graph::RenderGraphBuilder) {
    let output_image = rgb.get_swapchain_image_resource();

    const MAX_QUAD_COUNT: usize = u16::MAX as usize;
    const MAX_VERTEX_COUNT: usize = MAX_QUAD_COUNT * 4;
    const MAX_INDEX_COUNT: usize = MAX_QUAD_COUNT * 6;
    let vertex_buffer = rgb.create_buffer(render_graph::BufferResourceDescription::New(
        BufferDescription {
            size: MAX_VERTEX_COUNT * 20, //TODO: size of vertex
            usage: vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_location: gpu_allocator::MemoryLocation::CpuToGpu,
        },
    ));

    let index_buffer = rgb.create_buffer(render_graph::BufferResourceDescription::New(
        BufferDescription {
            size: MAX_INDEX_COUNT * std::mem::size_of::<u16>(),
            usage: vk::BufferUsageFlags::INDEX_BUFFER,
            memory_location: gpu_allocator::MemoryLocation::CpuToGpu,
        },
    ));

    let mut imgui_pass = rgb.create_pass("ImguiPass");
    let _ = imgui_pass.buffer(vertex_buffer, render_graph::BufferAccessType::VertexBuffer);
    let _ = imgui_pass.buffer(index_buffer, render_graph::BufferAccessType::IndexBuffer);
    imgui_pass.raster(vec![(output_image, [0.0, 0.5, 1.0, 0.0])], None);
}
