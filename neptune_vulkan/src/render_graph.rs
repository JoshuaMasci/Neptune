use crate::{AshDevice, BufferKey, ImageKey, PersistentResourceManager};
use ash::vk;
use std::collections::HashMap;

pub struct BufferAccess {
    write: bool, //TODO: calculate this from stage+access?
    stage: vk::PipelineStageFlags2,
    access: vk::AccessFlags2,
}

pub struct ImageAccess {
    pub write: bool, //TODO: calculate this from stage+access+layout?
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
    pub layout: vk::ImageLayout,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum BufferResource {
    Persistent(BufferKey),
    Transient(usize),
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum ImageResource {
    Persistent(ImageKey),
    Transient(usize),
    Swapchain(usize),
}

pub type BuildCommandFn = dyn Fn(
    &AshDevice,
    vk::CommandBuffer,
    &mut PersistentResourceManager,
    // &mut TransientResourceManager,
    // &SwapchainManager,
);

pub struct ColorAttachment {
    pub image: ImageResource,
    pub clear: Option<[f32; 4]>,
}

impl ColorAttachment {
    pub fn new(image: ImageResource) -> Self {
        Self { image, clear: None }
    }

    pub fn new_clear(image: ImageResource, clear: [f32; 4]) -> Self {
        Self {
            image,
            clear: Some(clear),
        }
    }
}

pub struct DepthStencilAttachment {
    pub image: ImageResource,
    pub clear: Option<(f32, u32)>,
}

impl DepthStencilAttachment {
    pub fn new(image: ImageResource) -> Self {
        Self { image, clear: None }
    }

    pub fn new_clear(image: ImageResource, clear: (f32, u32)) -> Self {
        Self {
            image,
            clear: Some(clear),
        }
    }
}

#[derive(Default)]
pub struct Framebuffer {
    pub color_attachments: Vec<ColorAttachment>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
    pub input_attachments: Vec<ImageResource>,
}

#[derive(Default)]
pub struct RenderPass {
    pub name: String,
    pub queue: vk::Queue,
    pub buffer_usages: HashMap<BufferResource, BufferAccess>,
    pub image_usages: HashMap<ImageResource, ImageAccess>,
    pub framebuffer: Option<Framebuffer>,
    pub build_cmd_fn: Option<Box<BuildCommandFn>>,
}

#[derive(Debug, Clone)]
pub struct TransientBufferDesc {
    size: vk::DeviceSize,
    memory_location: gpu_allocator::MemoryLocation,
}

#[derive(Debug, Clone)]
pub struct TransientImageDesc {
    extent: vk::Extent2D,
    format: vk::Format,
    memory_location: gpu_allocator::MemoryLocation,
}

#[derive(Default)]
pub struct RenderGraph {
    pub transient_buffers: Vec<TransientBufferDesc>,
    pub transient_images: Vec<TransientImageDesc>,
    pub swapchain_images: Vec<vk::SurfaceKHR>,
    pub passes: Vec<RenderPass>,
}

impl RenderGraph {
    pub fn create_transient_buffer(&mut self, desc: TransientBufferDesc) -> BufferResource {
        let index = self.transient_buffers.len();
        self.transient_buffers.push(desc);
        BufferResource::Transient(index)
    }

    pub fn create_transient_image(&mut self, desc: TransientImageDesc) -> ImageResource {
        let index = self.transient_images.len();
        self.transient_images.push(desc);
        ImageResource::Transient(index)
    }

    pub fn acquire_swapchain_image(&mut self, surface: vk::SurfaceKHR) -> ImageResource {
        let index = self.swapchain_images.len();
        self.swapchain_images.push(surface);
        ImageResource::Swapchain(index)
    }

    pub fn add_pass(&mut self, pass: RenderPass) {
        self.passes.push(pass);
    }
}

fn record_single_queue_render_graph_bad_sync(
    render_graph: &RenderGraph,
    device: &AshDevice,
    command_buffer: vk::CommandBuffer,
    resource_manager: &mut PersistentResourceManager,
) {
    for pass in render_graph.passes.iter() {
        //Bad Barrier
        unsafe {
            device.core.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfo::builder().memory_barriers(&[vk::MemoryBarrier2::builder()
                    .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
                    .build()]),
            );

            if let Some(build_cmd_fn) = &pass.build_cmd_fn {
                build_cmd_fn(device, command_buffer, resource_manager);
            }
        }
    }
}
