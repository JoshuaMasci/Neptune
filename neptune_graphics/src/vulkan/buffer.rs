use ash::vk;
use std::sync::{Arc, Mutex};

use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum AshBufferCreateError {
    #[error("Vk error: {0}")]
    VkError(ash::vk::Result),

    #[error("Gpu alloc error: {0}")]
    GpuAllocError(gpu_allocator::AllocationError),
}

pub(crate) struct AshBuffer {
    device: Arc<ash::Device>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    pub(crate) handle: vk::Buffer,
    pub(crate) allocation: gpu_allocator::vulkan::Allocation,
}

impl AshBuffer {
    pub(crate) fn new(
        device: Arc<ash::Device>,
        allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
        name: &str,
        create_info: &vk::BufferCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> Result<Self, AshBufferCreateError> {
        let handle = match unsafe { device.create_buffer(create_info, None) } {
            Ok(handle) => handle,
            Err(e) => return Err(AshBufferCreateError::VkError(e)),
        };

        let requirements = unsafe { device.get_buffer_memory_requirements(handle) };

        let allocation =
            match allocator
                .lock()
                .unwrap()
                .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                    name,
                    requirements,
                    location: memory_location,
                    linear: true,
                }) {
                Ok(allocation) => allocation,
                Err(e) => {
                    unsafe { device.destroy_buffer(handle, None) };
                    return Err(AshBufferCreateError::GpuAllocError(e));
                }
            };

        if let Err(e) =
            unsafe { device.bind_buffer_memory(handle, allocation.memory(), allocation.offset()) }
        {
            unsafe { device.destroy_buffer(handle, None) };
            let _ = allocator.lock().unwrap().free(allocation);
            return Err(AshBufferCreateError::VkError(e));
        }

        Ok(Self {
            device,
            allocator,
            handle,
            allocation,
        })
    }
}

impl Drop for AshBuffer {
    fn drop(&mut self) {
        unsafe { self.device.destroy_buffer(self.handle, None) };
        let _ = self
            .allocator
            .lock()
            .unwrap()
            .free(std::mem::take(&mut self.allocation));
    }
}
