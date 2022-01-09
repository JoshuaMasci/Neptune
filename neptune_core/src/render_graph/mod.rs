pub mod render_graph;

pub type BufferHandle = u32;
pub type ImageHandle = u32;

//Design for how the render_graph system might work
use crate::vulkan::{BufferDescription, ImageDescription};
use ash::vk;
pub fn build_render_graph_test() {
    let mut rgb = render_graph::RenderGraphBuilder::new();

    let imgui_image = build_imgui_pass(&mut rgb);

    let swapchain_image = rgb.get_swapchain_image_resource();
    build_blit_pass(&mut rgb, imgui_image, swapchain_image);

    let render_graph = rgb.build();
}

pub fn build_imgui_pass(rgb: &mut render_graph::RenderGraphBuilder) -> ImageHandle {
    let output_image = rgb.create_image(render_graph::ImageResource::New(ImageDescription {
        format: vk::Format::R8G8B8A8_UNORM,
        size: [1920, 1080],
        usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
        memory_location: gpu_allocator::MemoryLocation::GpuOnly,
    }));

    const MAX_QUAD_COUNT: usize = u16::MAX as usize;
    const MAX_VERTEX_COUNT: usize = MAX_QUAD_COUNT * 4;
    const MAX_INDEX_COUNT: usize = MAX_QUAD_COUNT * 6;
    let vertex_buffer = rgb.create_buffer(render_graph::BufferResource::New(BufferDescription {
        size: MAX_VERTEX_COUNT * 20, //TODO: size of vertex
        usage: vk::BufferUsageFlags::VERTEX_BUFFER,
        memory_location: gpu_allocator::MemoryLocation::CpuToGpu,
    }));

    let index_buffer = rgb.create_buffer(render_graph::BufferResource::New(BufferDescription {
        size: MAX_INDEX_COUNT * std::mem::size_of::<u16>(),
        usage: vk::BufferUsageFlags::INDEX_BUFFER,
        memory_location: gpu_allocator::MemoryLocation::CpuToGpu,
    }));

    let mut imgui_pass = rgb.create_pass("ImguiPass");
    imgui_pass.write_buffer(vertex_buffer, render_graph::BufferAccessType::VertexBuffer);
    imgui_pass.write_buffer(index_buffer, render_graph::BufferAccessType::IndexBuffer);
    imgui_pass.raster(vec![output_image], None);
    output_image
}

pub fn build_blit_pass(
    rgb: &mut render_graph::RenderGraphBuilder,
    src_image: ImageHandle,
    dst_image: ImageHandle,
) {
    let mut blit_pass = rgb.create_pass("SwapchainBlitPass");
    blit_pass.read_image(src_image, render_graph::ImageAccessType::BlitRead);
    blit_pass.write_image(dst_image, render_graph::ImageAccessType::BLitWrite);
}
