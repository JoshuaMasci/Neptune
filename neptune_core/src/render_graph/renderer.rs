use crate::render_backend::RenderDevice;
use crate::render_graph::compiled_pass::{BufferResource, ImageResource, RenderPassCompiled};
use crate::render_graph::render_graph::{
    BufferResourceDescription, ImageAccessType, ImageResourceDescription, RenderGraphDescription,
    RenderPassDescription,
};
use crate::vulkan::{Buffer, Image};
use ash::vk;
use std::rc::Rc;

pub struct Renderer {
    buffer_resources: Vec<Rc<Buffer>>,
    image_resources: Vec<Rc<Image>>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            buffer_resources: vec![],
            image_resources: vec![],
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
        let (buffers, images) = render_inline_temp(
            device,
            command_buffer,
            swapchain_image,
            swapchain_size,
            render_graph,
        );
        self.buffer_resources = buffers;
        self.image_resources = images;
    }
}

pub fn render_inline_temp(
    device: &RenderDevice,
    command_buffer: vk::CommandBuffer,
    swapchain_image: vk::Image,
    swapchain_size: vk::Extent2D,
    mut render_graph: RenderGraphDescription,
) -> (Vec<Rc<Buffer>>, Vec<Rc<Image>>) {
    //Compile Render Graph
    let (buffers, images) =
        create_resources(device, &render_graph, swapchain_image, swapchain_size);
    let mut compiled_passes: Vec<RenderPassCompiled> = render_graph
        .passes
        .iter_mut()
        .map(|render_pass| compile_render_pass(render_pass, &buffers, &images))
        .collect();

    let mut command_buffer_struct = crate::render_graph::CommandBuffer {
        device: device.base.clone(),
        command_buffer,
    };

    //Execute Passes
    for pass in compiled_passes.iter_mut() {
        block_all(device, command_buffer);
        transition_images(device, command_buffer, pass);
        if let Some(render_fn) = pass.render_fn.take() {
            render_fn(&mut command_buffer_struct, pass);
        }
    }
    (buffers, images)
}

//TODO: reuse resources
fn create_resources(
    device: &RenderDevice,
    render_graph: &RenderGraphDescription,
    swapchain_image: vk::Image,
    swapchain_size: vk::Extent2D,
) -> (Vec<Rc<Buffer>>, Vec<Rc<Image>>) {
    let buffers: Vec<Rc<Buffer>> = render_graph
        .buffers
        .iter()
        .map(|buffer_resource| match buffer_resource {
            BufferResourceDescription::New(buffer_description) => {
                Rc::new(Buffer::new(device, buffer_description))
            }
            BufferResourceDescription::Import(buffer) => buffer.clone(),
        })
        .collect();

    let images: Vec<Rc<Image>> = render_graph
        .images
        .iter()
        .map(|image_resource| match image_resource {
            //TODO: this better
            ImageResourceDescription::Swapchain => Rc::new(Image::from_existing_no_drop(
                device.base.clone(),
                device.allocator.clone(),
                swapchain_image,
                swapchain_size,
            )),
            ImageResourceDescription::New(image_description) => {
                Rc::new(Image::new_2d(device, image_description))
            }
            ImageResourceDescription::Import(image) => image.clone(),
        })
        .collect();

    (buffers, images)
}

fn compile_render_pass(
    render_pass: &mut RenderPassDescription,
    buffer_resources: &Vec<Rc<Buffer>>,
    image_resources: &Vec<Rc<Image>>,
) -> RenderPassCompiled {
    RenderPassCompiled {
        name: render_pass.name.clone(),
        read_buffers: render_pass
            .read_buffers
            .iter()
            .map(|buffer_access| BufferResource {
                buffer: buffer_resources[buffer_access.handle as usize].clone(),
                access_type: buffer_access.access_type,
            })
            .collect(),
        write_buffers: render_pass
            .write_buffers
            .iter()
            .map(|buffer_access| BufferResource {
                buffer: buffer_resources[buffer_access.handle as usize].clone(),
                access_type: buffer_access.access_type,
            })
            .collect(),
        read_images: render_pass
            .read_images
            .iter()
            .map(|image_access| ImageResource {
                image: image_resources[image_access.handle as usize].clone(),
                access_type: image_access.access_type,
            })
            .collect(),
        write_images: render_pass
            .write_images
            .iter()
            .map(|image_access| ImageResource {
                image: image_resources[image_access.handle as usize].clone(),
                access_type: image_access.access_type,
            })
            .collect(),
        pipelines: vec![],
        framebuffer: None,
        render_fn: render_pass.render_fn.take(),
    }
}

//TODO: track previous layout and only transition when needed
fn transition_images(
    device: &RenderDevice,
    command_buffer: vk::CommandBuffer,
    render_pass: &RenderPassCompiled,
) {
    let mut image_barriers: Vec<vk::ImageMemoryBarrier2KHR> = Vec::new();

    for read_image in render_pass.read_images.iter() {
        image_barriers.push(
            vk::ImageMemoryBarrier2KHR::builder()
                .image(read_image.image.image)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(temp_get_layout(read_image.access_type))
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

    for write_image in render_pass.write_images.iter() {
        image_barriers.push(
            vk::ImageMemoryBarrier2KHR::builder()
                .image(write_image.image.image)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(temp_get_layout(write_image.access_type))
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
        ImageAccessType::BlitRead => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        ImageAccessType::BLitWrite => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
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
