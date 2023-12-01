mod buffer;
mod debug_utils;
mod descriptor_set;
mod device;
mod image;
mod instance;
mod physical_device;
mod pipeline;
mod render_graph_executor;
mod resource_managers;
mod sampler;
mod swapchain;

mod compiled_render_graph_executor;
pub mod render_graph;
pub mod render_graph_builder;
pub mod render_graph_builder2;
mod render_graph_builder_trait;
mod upload_queue;

//Public Types
pub use ash::vk;
pub use gpu_allocator;

use crate::render_graph::BufferIndex;

pub use buffer::BufferDescription;
pub use device::{Device, DeviceSettings};
pub use image::{ImageDescription2D, TransientImageDesc, TransientImageSize};
pub use instance::{AppInfo, Instance};
pub use physical_device::*;
pub use pipeline::{
    ColorTargetState, DepthState, FragmentState, FramebufferDesc, PrimitiveState,
    RasterPipelineDescription, ShaderStage, VertexAttribute, VertexBufferLayout, VertexState,
};
pub use sampler::*;
pub use swapchain::SurfaceSettings;

slotmap::new_key_type! {
    pub struct SurfaceKey;
    pub struct BufferKey;
    pub struct ImageKey;
    pub struct SamplerKey;
    pub struct ComputePipelineKey;
    pub struct RasterPipleineKey;
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct SurfaceHandle(SurfaceKey);

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum BufferHandle {
    Persistent(BufferKey),
    Transient(BufferIndex),
}

impl BufferHandle {
    pub(crate) fn as_key(&self) -> BufferKey {
        match self {
            BufferHandle::Persistent(key) => *key,
            BufferHandle::Transient(_) => panic!("Cannot get a BufferKey from a transient buffer"),
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum ImageHandle {
    Persistent(ImageKey),
    Transient(usize),
}

impl ImageHandle {
    pub(crate) fn as_key(&self) -> ImageKey {
        match self {
            ImageHandle::Persistent(key) => *key,
            ImageHandle::Transient(_) => panic!("Cannot get a ImageKey from a transient image"),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SamplerHandle(SamplerKey);

#[derive(Copy, Clone, Debug)]
pub struct ComputePipelineHandle(ComputePipelineKey);

#[derive(Copy, Clone, Debug)]
pub struct RasterPipelineHandle(RasterPipleineKey);

#[derive(thiserror::Error, Debug)]
pub enum VulkanError {
    #[error("Vulkan Error: {0}")]
    Vk(#[from] vk::Result),
    #[error("GpuAllocator Error: {0}")]
    GpuAllocator(#[from] gpu_allocator::AllocationError),
}

/// Similar to promise/future in c++ and rust async. The contained type will be available sometime later
/// User should check it once a frame to see if it's ready.
pub struct VulkanFuture<T> {
    _phantom: std::marker::PhantomData<T>,
}
