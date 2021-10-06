mod buffer;
pub mod command_buffer;
mod descriptor_set;
pub mod device;
mod id_pool;
mod image;
pub mod instance;
mod pipeline;
pub mod render_task;
pub mod swapchain;

pub use ash::*;
pub use gpu_allocator;

pub use crate::instance::AppInfo;
pub use crate::instance::AppVersion;
pub use crate::instance::Instance;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct BufferId(u32);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ImageId(pub u32);

pub const SwapchainImageId: ImageId = ImageId(u32::MAX);
