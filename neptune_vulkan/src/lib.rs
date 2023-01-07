mod buffer;
mod compute_pipeline;
mod debug_utils;
mod descriptor_set;
mod device;
mod instance;
mod render_graph;
mod resource_manager;
mod sampler;
mod surface;
mod texture;
mod transfer_queue;

pub use buffer::*;
pub use device::*;
pub use instance::*;
pub use sampler::*;
use std::sync::{Arc, Mutex};
pub use texture::*;

pub use ash;
use slotmap::SlotMap;

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

pub type Result<T> = std::result::Result<T, Error>;

slotmap::new_key_type! {
    pub struct SurfaceHandle;
    pub struct SwapchainHandle;

    pub struct BufferHandle;
    pub struct TextureHandle;
    pub struct SamplerHandle;

    pub struct ComputePipelineHandle;
    pub struct RasterPipelineHandle;
}

pub struct GpuResource<K: slotmap::Key, T> {
    pub(crate) handle: K,
    pub(crate) list: Arc<Mutex<SlotMap<K, T>>>,
}

// impl<K: slotmap::Key, T> GpuResource<K, T> {
//     pub(crate) fn get(&self) -> &T {
//         self.list.lock().unwrap().get(self.handle).unwrap()
//     }
//
//     pub(crate) fn get_mut(&self) -> &mut T {
//         self.list.lock().unwrap().get_mut(self.handle).unwrap()
//     }
// }

impl<K: slotmap::Key, T> Drop for GpuResource<K, T> {
    fn drop(&mut self) {
        let _ = self
            .list
            .lock()
            .unwrap()
            .remove(self.handle)
            .expect("Failed to find key in slotmap");
    }
}
