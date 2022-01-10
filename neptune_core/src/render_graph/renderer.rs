use crate::render_backend::RenderDevice;
use crate::render_graph::render_graph::{
    BufferResourceDescription, ImageAccessType, ImageResourceDescription, RenderGraphDescription,
    RenderPassDescription,
};
use crate::render_graph::{RenderApi, RenderGraphResources, RenderPassInfo};
use crate::vulkan::{Buffer, Image, ImageDescription};
use ash::vk;

pub struct Renderer {
    resources: RenderGraphResources,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            resources: RenderGraphResources {
                buffers: vec![],
                images: vec![],
            },
        }
    }

    pub fn render(
        &mut self,
        device: &RenderDevice,
        command_buffer: vk::CommandBuffer,
        swapchain_image: vk::Image,
        swapchain_size: vk::Extent2D,
        mut render_graph: RenderGraphDescription,
    ) {
        self.resources = render_inline_temp(
            device,
            command_buffer,
            swapchain_image,
            swapchain_size,
            render_graph,
        );
    }
}

//Rendering A Render Graph
//Step 1:
//Step 2:
//Step 3: Pipeline barriers
//Step 4: Start RenderPass if needed
//Step 5: Call pass render function

pub fn render_inline_temp(
    device: &RenderDevice,
    command_buffer: vk::CommandBuffer,
    swapchain_image: vk::Image,
    swapchain_size: vk::Extent2D,
    mut render_graph: RenderGraphDescription,
) -> RenderGraphResources {
    //Compile Render Graph
    let resources = create_resources(device, &render_graph, swapchain_image, swapchain_size);

    // let mut compiled_passes: Vec<RenderPassCompiled> = render_graph
    //     .passes
    //     .iter_mut()
    //     .map(|render_pass| compile_render_pass(render_pass, &buffers, &images))
    //     .collect();
    //
    // let mut command_buffer_struct = crate::render_graph::CommandBuffer {
    //     device: device.base.clone(),
    //     command_buffer,
    // };
    //

    let mut render_api = RenderApi {
        device: device.clone(),
        command_buffer: command_buffer.clone(),
    };

    //Execute Passes
    for pass in render_graph.passes.iter_mut() {
        block_all(device, command_buffer);
        transition_images(device, command_buffer, pass, &resources);

        let render_pass_info = RenderPassInfo {
            name: pass.name.clone(),
            pipelines: vec![],
            frame_buffer_size: None,
        };

        if let Some(render_fn) = pass.render_fn.take() {
            render_fn(&mut render_api, &render_pass_info, &resources);
        }
    }
    resources
}

fn calculate_sync_stuff(render_graph: &RenderGraphDescription) {
    println!("Graph Begin-----------");

    for pass in render_graph.passes.iter() {
        println!("Pass: {} -----------", pass.name);
        for image_access in pass.images_dependencies.iter() {
            println!(
                "Use: {} {:?}",
                image_access.handle, image_access.access_type
            );
        }
        println!("--------------\n")
    }

    println!("Graph End-----------");
}

//TODO: reuse resources
fn create_resources(
    device: &RenderDevice,
    render_graph: &RenderGraphDescription,
    swapchain_image: vk::Image,
    swapchain_size: vk::Extent2D,
) -> RenderGraphResources {
    RenderGraphResources {
        buffers: render_graph
            .buffers
            .iter()
            .map(|buffer_resource| match buffer_resource {
                BufferResourceDescription::New(buffer_description) => {
                    Buffer::new(device, buffer_description.clone())
                }
                BufferResourceDescription::Import(buffer) => buffer.clone_no_drop(),
            })
            .collect(),
        images: render_graph
            .images
            .iter()
            .map(|image_resource| match image_resource {
                ImageResourceDescription::Swapchain => Image::from_existing(
                    ImageDescription {
                        format: vk::Format::UNDEFINED,
                        size: [swapchain_size.width, swapchain_size.height],
                        usage: Default::default(),
                        memory_location: gpu_allocator::MemoryLocation::Unknown,
                    },
                    swapchain_image,
                ),
                ImageResourceDescription::New(image_description) => {
                    Image::new(device, image_description.clone())
                }
                ImageResourceDescription::Import(image) => image.clone_no_drop(),
            })
            .collect(),
    }
}

// fn compile_render_pass(
//     render_pass: &mut RenderPassDescription,
//     buffer_resources: &Vec<Rc<Buffer>>,
//     image_resources: &Vec<Rc<Image>>,
// ) -> RenderPassCompiled {
//     RenderPassCompiled {
//         name: render_pass.name.clone(),
//         read_buffers: render_pass
//             .read_buffers
//             .iter()
//             .map(|buffer_access| BufferResource {
//                 buffer: buffer_resources[buffer_access.handle as usize].clone(),
//                 access_type: buffer_access.access_type,
//             })
//             .collect(),
//         write_buffers: render_pass
//             .write_buffers
//             .iter()
//             .map(|buffer_access| BufferResource {
//                 buffer: buffer_resources[buffer_access.handle as usize].clone(),
//                 access_type: buffer_access.access_type,
//             })
//             .collect(),
//         read_images: render_pass
//             .read_images
//             .iter()
//             .map(|image_access| ImageResource {
//                 image: image_resources[image_access.handle as usize].clone(),
//                 access_type: image_access.access_type,
//             })
//             .collect(),
//         write_images: render_pass
//             .write_images
//             .iter()
//             .map(|image_access| ImageResource {
//                 image: image_resources[image_access.handle as usize].clone(),
//                 access_type: image_access.access_type,
//             })
//             .collect(),
//         pipelines: vec![],
//         framebuffer: None,
//         render_fn: render_pass.render_fn.take(),
//     }
// }

//TODO: track previous layout and only transition when needed
fn transition_images(
    device: &RenderDevice,
    command_buffer: vk::CommandBuffer,
    render_pass: &RenderPassDescription,
    resources: &RenderGraphResources,
) {
    let mut image_barriers: Vec<vk::ImageMemoryBarrier2KHR> = Vec::new();

    for image in render_pass.images_dependencies.iter() {
        let image_handle = resources.images[image.handle as usize].handle;
        image_barriers.push(
            vk::ImageMemoryBarrier2KHR::builder()
                .image(image_handle)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(temp_get_layout(image.access_type))
                .src_access_mask(vk::AccessFlags2KHR::NONE)
                .src_stage_mask(vk::PipelineStageFlags2KHR::NONE)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_access_mask(vk::AccessFlags2KHR::NONE)
                .dst_stage_mask(vk::PipelineStageFlags2KHR::NONE)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_array_layer(0)
                        .layer_count(1)
                        .base_mip_level(0)
                        .level_count(1)
                        .build(),
                )
                .build(),
        );
    }

    unsafe {
        device.synchronization2.cmd_pipeline_barrier2(
            command_buffer,
            &vk::DependencyInfoKHR::builder().image_memory_barriers(&image_barriers),
        );
    }
}

fn temp_get_layout(access_type: ImageAccessType) -> vk::ImageLayout {
    match access_type {
        ImageAccessType::TransferRead => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        ImageAccessType::TransferWrite => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        ImageAccessType::ColorAttachmentRead => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        ImageAccessType::ColorAttachmentWrite => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        ImageAccessType::DepthStencilAttachmentRead => {
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
        }
        ImageAccessType::DepthStencilAttachmentWrite => {
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
        }
        ImageAccessType::ShaderComputeRead => vk::ImageLayout::GENERAL,
        ImageAccessType::ShaderComputeWrite => vk::ImageLayout::GENERAL,
        _ => vk::ImageLayout::GENERAL,
    }
}

//Absolutely the worst way of doing this, DO NOT LEAVE THIS IN!!!
//TODO: figure out correct syncing
fn block_all(device: &RenderDevice, command_buffer: vk::CommandBuffer) {
    unsafe {
        device.base.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::DEVICE_GROUP,
            &[],
            &[],
            &[],
        );
    }
}

//TODO: use?
// struct RenderGraph {
//     device: RenderDevice,
//     buffer_resources: Vec<Rc<Buffer>>,
//     image_resources: Vec<Rc<Image>>,
//     passes: Vec<RenderPassCompiled>,
// }
// struct PipelineCache {}
