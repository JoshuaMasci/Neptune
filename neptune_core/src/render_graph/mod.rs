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

    let imgui_image = build_imgui_pass(&mut rgb);

    let swapchain_image = rgb.get_swapchain_image_resource();
    build_blit_pass(&mut rgb, imgui_image, swapchain_image);

    rgb.build()
}

pub fn build_imgui_pass(rgb: &mut render_graph::RenderGraphBuilder) -> ImageHandle {
    let output_image = rgb.create_image(render_graph::ImageResourceDescription::New(
        ImageDescription {
            format: vk::Format::R8G8B8A8_UNORM,
            size: [1920, 1080],
            usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
            memory_location: gpu_allocator::MemoryLocation::GpuOnly,
        },
    ));

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
    output_image
}

pub fn build_blit_pass(
    rgb: &mut render_graph::RenderGraphBuilder,
    src_image: ImageHandle,
    dst_image: ImageHandle,
) {
    let mut blit_pass = rgb.create_pass("SwapchainBlitPass");
    blit_pass.image(src_image, render_graph::ImageAccessType::TransferRead);
    blit_pass.image(dst_image, render_graph::ImageAccessType::TransferWrite);
    blit_pass.render(move |render_api, pass_info, resources| {
        let image_layers = vk::ImageSubresourceLayers::builder()
            .base_array_layer(0)
            .layer_count(1)
            .mip_level(0)
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .build();

        let src_image = &resources.images[src_image as usize];
        let dst_image = &resources.images[dst_image as usize];

        unsafe {
            render_api.device.base.cmd_blit_image(
                render_api.command_buffer,
                src_image.handle,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                dst_image.handle,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[vk::ImageBlit::builder()
                    .src_offsets([
                        vk::Offset3D { x: 0, y: 0, z: 0 },
                        vk::Offset3D {
                            x: src_image.description.size[0] as i32,
                            y: src_image.description.size[1] as i32,
                            z: 1,
                        },
                    ])
                    .dst_offsets([
                        vk::Offset3D { x: 0, y: 0, z: 0 },
                        vk::Offset3D {
                            x: dst_image.description.size[0] as i32,
                            y: dst_image.description.size[1] as i32,
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
