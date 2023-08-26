mod buffer;
mod debug_utils;
mod descriptor_set;
mod device;
mod image;
mod instance;
mod interface;
mod pipeline;
mod render_graph;
mod resource_managers;
mod swapchain;

//Public Types
pub use ash::vk;
pub use gpu_allocator;

pub use buffer::BufferDescription;
pub use device::{Device, DeviceSettings};
pub use instance::AppInfo;
pub use instance::Instance;
pub use pipeline::{
    ColorTargetState, DepthState, FragmentState, FramebufferDesc, PrimitiveState,
    RasterPipelineDescription, ShaderStage, VertexAttribute, VertexBufferLayout, VertexState,
};
pub use render_graph::*;
pub use swapchain::SurfaceSettings;

slotmap::new_key_type! {
    pub struct SurfaceKey;
    pub struct BufferKey;
    pub struct ImageKey;
    pub struct ComputePipelineKey;
    pub struct RasterPipleineKey;
}

#[derive(Copy, Clone, Debug)]
pub struct SurfaceHandle(SurfaceKey);

// #[derive(Copy, Clone, Debug)]
// pub struct BufferHandle(BufferKey);

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum BufferHandle {
    Persistent(BufferKey),
    Transient(usize),
}

// #[derive(Copy, Clone, Debug)]
// pub struct ImageHandle(ImageKey);

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum ImageHandle {
    Persistent(ImageKey),
    Transient(usize),
    Swapchain(usize),
}

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
