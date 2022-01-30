mod pipeline_cache;
pub mod render_graph;
mod renderer;

use crate::render_backend::RenderDevice;
use crate::render_graph::render_graph::RenderGraphDescription;
pub use crate::render_graph::renderer::Renderer;
use crate::vulkan::{Buffer, Image};

use crate::transfer_queue::TransferQueue;
use ash::vk;

pub type BufferHandle = u32;
pub type ImageHandle = u32;

pub type RenderFn =
    dyn FnOnce(&mut RenderApi, &mut TransferQueue, &RenderPassInfo, &RenderGraphResources);

pub struct RenderApi {
    pub device: RenderDevice,
    pub command_buffer: vk::CommandBuffer,
}

pub struct RenderPassInfo {
    pub name: String,
    pub pipelines: Vec<vk::Pipeline>,
    pub framebuffer_size: Option<vk::Extent2D>,
}

pub struct RenderGraphResources {
    pub(crate) buffers: Vec<Buffer>,
    pub(crate) images: Vec<Image>,
}
