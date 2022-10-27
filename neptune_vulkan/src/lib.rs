mod buffer;
mod device;
mod image;
mod instance;

pub use buffer::*;
pub use device::*;
pub use image::*;
pub use instance::*;

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
