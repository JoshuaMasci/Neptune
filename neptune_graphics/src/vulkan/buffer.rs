use crate::vulkan::device::{AshDevice, VulkanCreateError};
use ash::vk;
use std::sync::Arc;

pub(crate) struct AshBuffer {
    device: Arc<AshDevice>,
    pub(crate) handle: vk::Buffer,
    pub(crate) allocation: gpu_allocator::vulkan::Allocation,
}

impl AshBuffer {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        name: &str,
        create_info: &vk::BufferCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> Result<Self, VulkanCreateError> {
        let handle = unsafe { device.core.create_buffer(create_info, None)? };

        let requirements = unsafe { device.core.get_buffer_memory_requirements(handle) };

        let allocation = match device.allocator.lock().unwrap().allocate(
            &gpu_allocator::vulkan::AllocationCreateDesc {
                name,
                requirements,
                location: memory_location,
                linear: true,
            },
        ) {
            Ok(allocation) => allocation,
            Err(e) => {
                unsafe { device.core.destroy_buffer(handle, None) };
                return Err(VulkanCreateError::GpuAllocError(e));
            }
        };

        if let Err(e) = unsafe {
            device
                .core
                .bind_buffer_memory(handle, allocation.memory(), allocation.offset())
        } {
            unsafe { device.core.destroy_buffer(handle, None) };
            let _ = device.allocator.lock().unwrap().free(allocation);
            return Err(VulkanCreateError::VkError(e));
        }

        Ok(Self {
            device,
            handle,
            allocation,
        })
    }
}

impl Drop for AshBuffer {
    fn drop(&mut self) {
        unsafe { self.device.core.destroy_buffer(self.handle, None) };
        let _ = self
            .device
            .allocator
            .lock()
            .unwrap()
            .free(std::mem::take(&mut self.allocation));
    }
}
