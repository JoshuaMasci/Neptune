mod buffer;
mod debug_messenger;
mod descriptor_set;
mod device;
mod graph;
mod instance;
mod pipeline_cache;
mod raster_api;
mod shader;
mod swapchain;
mod texture;

pub use graph::RasterFnVulkan;
pub use pipeline_cache::PipelineCache;

pub use buffer::Buffer;
pub use device::Device;
pub use instance::Instance;
pub use pipeline_cache::FramebufferLayout;
pub use shader::ShaderModule;
pub use texture::Texture;

pub(crate) use graph::Graph;

use crate::render_graph::{BufferAccess, TextureAccess};
use crate::{BufferDescription, TextureDescription};
use ash::vk;

impl BufferAccess {
    pub(crate) fn get_vk(&self) -> (vk::PipelineStageFlags2, vk::AccessFlags2) {
        match self {
            BufferAccess::None => (vk::PipelineStageFlags2::NONE, vk::AccessFlags2::NONE),
            BufferAccess::IndexBufferRead => (
                vk::PipelineStageFlags2::INDEX_INPUT,
                vk::AccessFlags2::MEMORY_READ,
            ),
            BufferAccess::VertexBufferRead => (
                vk::PipelineStageFlags2::VERTEX_INPUT,
                vk::AccessFlags2::MEMORY_READ,
            ),
            BufferAccess::TransferRead => (
                vk::PipelineStageFlags2::TRANSFER,
                vk::AccessFlags2::TRANSFER_READ,
            ),
            BufferAccess::TransferWrite => (
                vk::PipelineStageFlags2::TRANSFER,
                vk::AccessFlags2::TRANSFER_WRITE,
            ),
            BufferAccess::ShaderRead => (
                vk::PipelineStageFlags2::ALL_GRAPHICS,
                vk::AccessFlags2::SHADER_STORAGE_READ,
            ),
            BufferAccess::ShaderWrite => (
                vk::PipelineStageFlags2::ALL_GRAPHICS,
                vk::AccessFlags2::SHADER_STORAGE_WRITE,
            ),
        }
    }
}

impl TextureAccess {
    pub(crate) fn get_vk(&self) -> (vk::PipelineStageFlags2, vk::AccessFlags2, vk::ImageLayout) {
        match self {
            TextureAccess::None => (
                vk::PipelineStageFlags2KHR::NONE,
                vk::AccessFlags2KHR::NONE,
                vk::ImageLayout::UNDEFINED,
            ),
            TextureAccess::ColorAttachmentWrite => (
                vk::PipelineStageFlags2KHR::COLOR_ATTACHMENT_OUTPUT,
                vk::AccessFlags2KHR::COLOR_ATTACHMENT_WRITE,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            ),
            TextureAccess::DepthStencilAttachmentWrite => (
                vk::PipelineStageFlags2KHR::EARLY_FRAGMENT_TESTS
                    | vk::PipelineStageFlags2KHR::LATE_FRAGMENT_TESTS,
                vk::AccessFlags2KHR::DEPTH_STENCIL_ATTACHMENT_WRITE,
                vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            ),
            TextureAccess::TransferRead => (
                vk::PipelineStageFlags2KHR::TRANSFER,
                vk::AccessFlags2KHR::TRANSFER_READ,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            ),
            TextureAccess::TransferWrite => (
                vk::PipelineStageFlags2KHR::TRANSFER,
                vk::AccessFlags2KHR::TRANSFER_WRITE,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            ),
            TextureAccess::ShaderSampledRead => (
                vk::PipelineStageFlags2KHR::ALL_GRAPHICS,
                vk::AccessFlags2KHR::SHADER_SAMPLED_READ,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            ),
            TextureAccess::ShaderStorageRead => (
                vk::PipelineStageFlags2KHR::ALL_GRAPHICS,
                vk::AccessFlags2KHR::SHADER_STORAGE_READ,
                vk::ImageLayout::GENERAL,
            ),
            TextureAccess::ShaderStorageWrite => (
                vk::PipelineStageFlags2KHR::ALL_GRAPHICS,
                vk::AccessFlags2KHR::SHADER_STORAGE_WRITE,
                vk::ImageLayout::GENERAL,
            ),
        }
    }
}
