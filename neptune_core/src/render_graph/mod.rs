mod compiled_pass;
pub mod render_graph;
mod renderer;

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
    let _ = imgui_pass.write_buffer(vertex_buffer, render_graph::BufferAccessType::VertexBuffer);
    let _ = imgui_pass.write_buffer(index_buffer, render_graph::BufferAccessType::IndexBuffer);
    imgui_pass.raster(vec![output_image], None);
    output_image
}

pub fn build_blit_pass(
    rgb: &mut render_graph::RenderGraphBuilder,
    src_image: ImageHandle,
    dst_image: ImageHandle,
) {
    let mut blit_pass = rgb.create_pass("SwapchainBlitPass");
    let src_index = blit_pass.read_image(src_image, render_graph::ImageAccessType::BlitRead);
    let dst_index = blit_pass.write_image(dst_image, render_graph::ImageAccessType::BLitWrite);

    blit_pass.render(move |command_buffer, compiled_pass| {
        let image_layers = vk::ImageSubresourceLayers::builder()
            .base_array_layer(0)
            .layer_count(1)
            .mip_level(0)
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .build();

        let src_image = &compiled_pass.read_images[src_index];
        let dst_image = &compiled_pass.write_images[dst_index];

        unsafe {
            command_buffer.device.cmd_blit_image(
                command_buffer.command_buffer,
                src_image.image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                dst_image.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[vk::ImageBlit::builder()
                    .src_offsets([
                        vk::Offset3D { x: 0, y: 0, z: 0 },
                        vk::Offset3D {
                            x: src_image.size.width as i32,
                            y: src_image.size.height as i32,
                            z: 1,
                        },
                    ])
                    .dst_offsets([
                        vk::Offset3D { x: 0, y: 0, z: 0 },
                        vk::Offset3D {
                            x: dst_image.size.width as i32,
                            y: dst_image.size.height as i32,
                            z: 1,
                        },
                    ])
                    .src_subresource(image_layers)
                    .dst_subresource(image_layers)
                    .build()],
                vk::Filter::NEAREST,
            );
        }
    });
}
