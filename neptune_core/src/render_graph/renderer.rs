use crate::render_backend::RenderDevice;
use crate::render_graph::render_graph::{
    BufferAccessType, BufferResourceDescription, ImageAccessType, ImageResourceDescription,
    RenderGraphDescription, RenderPassDescription,
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

pub fn render_inline_temp(
    device: &RenderDevice,
    command_buffer: vk::CommandBuffer,
    swapchain_image: vk::Image,
    swapchain_size: vk::Extent2D,
    mut render_graph: RenderGraphDescription,
) -> RenderGraphResources {
    let resources = create_resources(device, &render_graph, swapchain_image, swapchain_size);
    let mut previous_buffer_state: Vec<BufferAccessType> = resources
        .buffers
        .iter()
        .map(|_| BufferAccessType::None)
        .collect();
    let mut previous_image_state: Vec<ImageAccessType> = resources
        .images
        .iter()
        .map(|_| ImageAccessType::None)
        .collect();

    let mut render_api = RenderApi {
        device: device.clone(),
        command_buffer: command_buffer.clone(),
    };

    //Execute Passes
    for pass in render_graph.passes.iter_mut() {
        buffer_barriers(
            device,
            command_buffer,
            pass,
            &resources,
            &mut previous_buffer_state,
        );

        image_barriers(
            device,
            command_buffer,
            pass,
            &resources,
            &mut previous_image_state,
        );

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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct BarrierFlags {
    stage: vk::PipelineStageFlags2KHR,
    access: vk::AccessFlags2KHR,
}

fn get_buffer_barrier_flags(buffer_access: BufferAccessType) -> BarrierFlags {
    match buffer_access {
        BufferAccessType::None => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::NONE,
            access: vk::AccessFlags2KHR::NONE,
        },
        BufferAccessType::IndexBuffer => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::INDEX_INPUT,
            access: vk::AccessFlags2KHR::MEMORY_READ,
        },
        BufferAccessType::VertexBuffer => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::VERTEX_INPUT,
            access: vk::AccessFlags2KHR::MEMORY_READ,
        },
        BufferAccessType::TransferRead => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::TRANSFER,
            access: vk::AccessFlags2KHR::TRANSFER_READ,
        },
        BufferAccessType::TransferWrite => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::TRANSFER,
            access: vk::AccessFlags2KHR::TRANSFER_WRITE,
        },
        BufferAccessType::ShaderRead => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::ALL_GRAPHICS,
            access: vk::AccessFlags2KHR::SHADER_STORAGE_READ,
        },
        BufferAccessType::ShaderWrite => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::ALL_GRAPHICS,
            access: vk::AccessFlags2KHR::SHADER_STORAGE_WRITE,
        },
    }
}

fn get_image_barrier_flags(image_access: ImageAccessType) -> (vk::ImageLayout, BarrierFlags) {
    match image_access {
        ImageAccessType::None => (
            vk::ImageLayout::UNDEFINED,
            BarrierFlags {
                stage: vk::PipelineStageFlags2KHR::NONE,
                access: vk::AccessFlags2KHR::NONE,
            },
        ),
        ImageAccessType::TransferRead => (
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            BarrierFlags {
                stage: vk::PipelineStageFlags2KHR::TRANSFER,
                access: vk::AccessFlags2KHR::TRANSFER_READ,
            },
        ),
        ImageAccessType::TransferWrite => (
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            BarrierFlags {
                stage: vk::PipelineStageFlags2KHR::TRANSFER,
                access: vk::AccessFlags2KHR::TRANSFER_WRITE,
            },
        ),
        ImageAccessType::ShaderSampledRead => (
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            BarrierFlags {
                stage: vk::PipelineStageFlags2KHR::ALL_GRAPHICS,
                access: vk::AccessFlags2KHR::SHADER_SAMPLED_READ,
            },
        ),
        ImageAccessType::ShaderStorageRead => (
            vk::ImageLayout::GENERAL,
            BarrierFlags {
                stage: vk::PipelineStageFlags2KHR::ALL_GRAPHICS,
                access: vk::AccessFlags2KHR::SHADER_STORAGE_READ,
            },
        ),
        ImageAccessType::ShaderStorageWrite => (
            vk::ImageLayout::GENERAL,
            BarrierFlags {
                stage: vk::PipelineStageFlags2KHR::ALL_GRAPHICS,
                access: vk::AccessFlags2KHR::SHADER_STORAGE_WRITE,
            },
        ),
        ImageAccessType::ColorAttachmentRead
        | ImageAccessType::ColorAttachmentWrite
        | ImageAccessType::DepthStencilAttachmentRead
        | ImageAccessType::DepthStencilAttachmentWrite => {
            panic!("Attachment Barrier/Transitions should be handled by the render-pass")
        }
    }
}

/// Waits on the last usage of the buffers before the next pass executes.
/// This will be suboptimal as it will not allow parallel reads of a buffer.
/// TODO: allow parallel reads!
fn buffer_barriers(
    device: &RenderDevice,
    command_buffer: vk::CommandBuffer,
    render_pass: &RenderPassDescription,
    resources: &RenderGraphResources,
    buffer_state: &mut Vec<BufferAccessType>,
) {
    let buffer_barriers: Vec<vk::BufferMemoryBarrier2KHR> = render_pass
        .buffers_dependencies
        .iter()
        .map(|buffer_access| {
            let src_flags = get_buffer_barrier_flags(buffer_state[buffer_access.handle as usize]);
            let dst_flags = get_buffer_barrier_flags(buffer_access.access_type);
            buffer_state[buffer_access.handle as usize] = buffer_access.access_type;
            vk::BufferMemoryBarrier2KHR::builder()
                .buffer(resources.buffers[buffer_access.handle as usize].handle)
                .size(vk::WHOLE_SIZE)
                .src_stage_mask(src_flags.stage)
                .src_access_mask(src_flags.access)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_stage_mask(dst_flags.stage)
                .dst_access_mask(dst_flags.access)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .build()
        })
        .collect();

    unsafe {
        device.synchronization2.cmd_pipeline_barrier2(
            command_buffer,
            &vk::DependencyInfoKHR::builder().buffer_memory_barriers(&buffer_barriers),
        );
    }
}

/// Waits on the last usage of the images before the next pass executes.
/// This will be suboptimal as it will not allow parallel reads of a image.
/// TODO: allow parallel reads!
fn image_barriers(
    device: &RenderDevice,
    command_buffer: vk::CommandBuffer,
    render_pass: &RenderPassDescription,
    resources: &RenderGraphResources,
    image_state: &mut Vec<ImageAccessType>,
) {
    let image_barriers: Vec<vk::ImageMemoryBarrier2KHR> = render_pass
        .images_dependencies
        .iter()
        .map(|image_access| {
            let src_flags = get_image_barrier_flags(image_state[image_access.handle as usize]);
            let dst_flags = get_image_barrier_flags(image_access.access_type);
            image_state[image_access.handle as usize] = image_access.access_type;
            vk::ImageMemoryBarrier2KHR::builder()
                .image(resources.images[image_access.handle as usize].handle)
                .old_layout(src_flags.0)
                .new_layout(dst_flags.0)
                .src_stage_mask(src_flags.1.stage)
                .src_access_mask(src_flags.1.access)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_stage_mask(dst_flags.1.stage)
                .dst_access_mask(dst_flags.1.access)
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
                .build()
        })
        .collect();

    unsafe {
        device.synchronization2.cmd_pipeline_barrier2(
            command_buffer,
            &vk::DependencyInfoKHR::builder().image_memory_barriers(&image_barriers),
        );
    }
}
