mod buffer;
mod device;
mod image;
mod instance;

pub use buffer::*;
pub use device::*;
pub use image::*;
pub use instance::*;

pub use ash;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum MemoryType {
    GpuOnly,
    CpuToGpu,
    GpuToCpu,
}

impl MemoryType {
    pub(crate) fn to_gpu_alloc(self) -> gpu_allocator::MemoryLocation {
        match self {
            MemoryType::GpuOnly => gpu_allocator::MemoryLocation::GpuOnly,
            MemoryType::CpuToGpu => gpu_allocator::MemoryLocation::CpuToGpu,
            MemoryType::GpuToCpu => gpu_allocator::MemoryLocation::GpuToCpu,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum NeptuneVulkanError {
    #[error("Vk error: {0}")]
    VkError(ash::vk::Result),

    #[error("Gpu alloc error: {0}")]
    GpuAllocError(gpu_allocator::AllocationError),

    #[error("Error: {0}")]
    StringError(String),
}

pub type Result<T> = std::result::Result<T, NeptuneVulkanError>;
