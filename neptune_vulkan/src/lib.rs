mod bindless_descriptor;
mod buffer;
mod device;
mod instance;
mod resource_manager;
mod sampler;
mod texture;

pub use buffer::*;
pub use device::*;
pub use instance::*;
pub use sampler::*;
pub use texture::*;

pub use ash;

#[macro_use]
extern crate log;

pub type MemoryLocation = gpu_allocator::MemoryLocation;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Vk error: {0}")]
    VkError(ash::vk::Result),

    #[error("Gpu alloc error: {0}")]
    GpuAllocError(gpu_allocator::AllocationError),

    #[error("Error: {0}")]
    StringError(String),
}

impl Error {
    pub(crate) fn string(s: &str) -> Self {
        self::Error::StringError(String::from(s))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
