pub mod pipeline_cache;
pub(crate) mod render_graph;
mod renderer;

pub use crate::render_graph::renderer::Renderer;

use crate::render_backend::RenderDevice;
use crate::vulkan::{Buffer, Image};

use crate::render_graph::pipeline_cache::{FramebufferLayout, PipelineCache};
use crate::transfer_queue::TransferQueue;
use ash::vk;

pub type BufferHandle = u32;
pub type ImageHandle = u32;

pub type RenderFn = dyn FnOnce(
    &mut RenderApi,
    &mut PipelineCache,
    &mut TransferQueue,
    &RenderPassInfo,
    &RenderGraphResources,
);

pub struct RenderApi {
    pub device: RenderDevice,
    pub command_buffer: vk::CommandBuffer,
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_set: vk::DescriptorSet,
}

pub struct FramebufferInfo {
    pub size: vk::Extent2D,
    pub layout: FramebufferLayout,
}

pub struct RenderPassInfo {
    pub name: String,
    pub framebuffer: Option<FramebufferInfo>,
}

pub struct RenderGraphResources {
    pub(crate) buffers: Vec<Buffer>,
    pub(crate) images: Vec<Image>,
}
