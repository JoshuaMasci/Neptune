use crate::render_backend::RenderDevice;
use crate::render_graph::render_graph::{
    BufferAccessType, BufferResourceDescription, ImageAccessType, ImageResourceDescription,
    RenderGraphDescription, RenderPassDescription,
};
use crate::render_graph::{RenderApi, RenderGraphResources, RenderPassInfo};
use crate::transfer_queue::TransferQueue;
use crate::vulkan::{Buffer, Image};
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
        swapchain_image: Image,
        render_graph: RenderGraphDescription,
        transfer_queue: &mut crate::transfer_queue::TransferQueue,
    ) {
        self.resources = render_inline_temp(
            device,
            command_buffer,
            swapchain_image,
            render_graph,
            transfer_queue,
        );
    }
}

pub fn render_inline_temp(
    device: &RenderDevice,
    command_buffer: vk::CommandBuffer,
    swapchain_image: Image,
    mut render_graph: RenderGraphDescription,
    mut transfer_queue: &mut crate::transfer_queue::TransferQueue,
) -> RenderGraphResources {
    let resources = create_resources(device, &render_graph, swapchain_image);
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

        let mut frame_buffer_size: Option<vk::Extent2D> = None;

        if let Some(framebuffer) = &pass.framebuffer {
            let framebuffer_size: [u32; 2] = {
                //Verify size
                let mut framebuffer_size: Option<[u32; 2]> = None;
                for color_attachment_description in framebuffer.color_attachments.iter() {
                    let color_attachment =
                        &resources.images[color_attachment_description.0 as usize];
                    if let Some(size) = framebuffer_size {
                        if size != color_attachment.description.size {
                            panic!("Color attachment size doesn't match rest of framebuffer");
                        }
                    } else {
                        framebuffer_size = Some(color_attachment.description.size);
                    }
                }

                if let Some(depth_attachment_description) = &framebuffer.depth_attachment {
                    let depth_attachment =
                        &resources.images[depth_attachment_description.0 as usize];
                    if let Some(size) = framebuffer_size {
                        if size != depth_attachment.description.size {
                            panic!("Depth attachment size doesn't match rest of framebuffer");
                        }
                    } else {
                        framebuffer_size = Some(depth_attachment.description.size);
                    }
                }

                framebuffer_size.expect("Framebuffer has no attachments")
            };

            frame_buffer_size = Some(vk::Extent2D {
                width: framebuffer_size[0],
                height: framebuffer_size[1],
            });

            let color_attachments: Vec<vk::RenderingAttachmentInfoKHR> = framebuffer
                .color_attachments
                .iter()
                .map(|color_attachment_description| {
                    let color_attachment =
                        &resources.images[color_attachment_description.0 as usize];

                    vk::RenderingAttachmentInfoKHR::builder()
                        .image_view(color_attachment.view)
                        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .load_op(vk::AttachmentLoadOp::CLEAR)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .clear_value(vk::ClearValue {
                            color: vk::ClearColorValue {
                                float32: color_attachment_description.1,
                            },
                        })
                        .build()
                })
                .collect();

            let mut rendering_info = vk::RenderingInfoKHR::builder()
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D {
                        width: framebuffer_size[0],
                        height: framebuffer_size[1],
                    },
                })
                .layer_count(1)
                .color_attachments(&color_attachments);

            let mut depth_attachment_info = vk::RenderingAttachmentInfoKHR::builder();
            if let Some(depth_attachment_description) = framebuffer.depth_attachment {
                let depth_attachment = &resources.images[depth_attachment_description.0 as usize];

                depth_attachment_info = depth_attachment_info
                    .image_view(depth_attachment.view)
                    .image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: depth_attachment_description.1,
                            stencil: 0,
                        },
                    });

                rendering_info = rendering_info.depth_attachment(&depth_attachment_info);
                rendering_info = rendering_info.stencil_attachment(&depth_attachment_info);
            }

            unsafe {
                device
                    .dynamic_rendering
                    .cmd_begin_rendering(command_buffer, &rendering_info);
            }
        }

        let render_pass_info = RenderPassInfo {
            name: pass.name.clone(),
            pipelines: vec![],
            framebuffer_size: frame_buffer_size,
        };

        if let Some(render_fn) = pass.render_fn.take() {
            render_fn(
                &mut render_api,
                &mut transfer_queue,
                &render_pass_info,
                &resources,
            );
        }

        if pass.framebuffer.is_some() {
            unsafe {
                device.dynamic_rendering.cmd_end_rendering(command_buffer);
            }
        }
    }

    //Transition swapchain image to present layout
    {
        let swapchain_image_handle = 0usize;
        let src_flags = get_image_barrier_flags(previous_image_state[swapchain_image_handle]);
        unsafe {
            device.synchronization2.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfoKHR::builder().image_memory_barriers(&[
                    vk::ImageMemoryBarrier2KHR::builder()
                        .image(resources.images[swapchain_image_handle].handle)
                        .old_layout(src_flags.0)
                        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                        .src_stage_mask(src_flags.1.stage)
                        .src_access_mask(src_flags.1.access)
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_stage_mask(vk::PipelineStageFlags2KHR::NONE)
                        .dst_access_mask(vk::AccessFlags2KHR::NONE)
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
                ]),
            );
        }
    }

    resources
}

//TODO: reuse resources
fn create_resources(
    device: &RenderDevice,
    render_graph: &RenderGraphDescription,
    swapchain_image: Image,
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
                ImageResourceDescription::Swapchain => swapchain_image.clone_no_drop(),
                ImageResourceDescription::New(image_description) => {
                    let mut image = Image::new(device, image_description.clone());
                    image.create_image_view();
                    image
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
        ImageAccessType::ColorAttachmentRead => (
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            BarrierFlags {
                stage: vk::PipelineStageFlags2KHR::ALL_GRAPHICS,
                access: vk::AccessFlags2KHR::COLOR_ATTACHMENT_READ,
            },
        ),
        ImageAccessType::ColorAttachmentWrite => (
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            BarrierFlags {
                stage: vk::PipelineStageFlags2KHR::COLOR_ATTACHMENT_OUTPUT,
                access: vk::AccessFlags2KHR::COLOR_ATTACHMENT_WRITE,
            },
        ),
        ImageAccessType::DepthStencilAttachmentRead => (
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            BarrierFlags {
                stage: vk::PipelineStageFlags2KHR::ALL_GRAPHICS,
                access: vk::AccessFlags2KHR::DEPTH_STENCIL_ATTACHMENT_READ,
            },
        ),
        ImageAccessType::DepthStencilAttachmentWrite => (
            vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
            BarrierFlags {
                stage: vk::PipelineStageFlags2KHR::EARLY_FRAGMENT_TESTS
                    | vk::PipelineStageFlags2KHR::LATE_FRAGMENT_TESTS,
                access: vk::AccessFlags2KHR::DEPTH_STENCIL_ATTACHMENT_WRITE,
            },
        ),
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
