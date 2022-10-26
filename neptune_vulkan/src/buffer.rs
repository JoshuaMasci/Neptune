use crate::{AshDevice, Error};
use ash::vk;
use std::sync::{Arc, Mutex};

pub struct Buffer {
    device: Arc<AshDevice>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,

    pub handle: vk::Buffer,
    pub allocation: gpu_allocator::vulkan::Allocation,
}

impl Buffer {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
        create_info: &vk::BufferCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> crate::Result<Self> {
        let handle = match unsafe { device.create_buffer(create_info, None) } {
            Ok(handle) => handle,
            Err(e) => return Err(Error::VkError(e)),
        };

        let requirements = unsafe { device.get_buffer_memory_requirements(handle) };

        let allocation =
            match allocator
                .lock()
                .unwrap()
                .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                    name: "Buffer Allocation",
                    requirements,
                    location: memory_location,
                    linear: true,
                }) {
                Ok(allocation) => allocation,
                Err(e) => {
                    unsafe { device.destroy_buffer(handle, None) };
                    return Err(Error::GpuAllocError(e));
                }
            };

        if let Err(e) =
            unsafe { device.bind_buffer_memory(handle, allocation.memory(), allocation.offset()) }
        {
            unsafe { device.destroy_buffer(handle, None) };
            let _ = allocator.lock().unwrap().free(allocation);
            return Err(Error::VkError(e));
        }

        Ok(Self {
            device,
            allocator,
            allocation,
            handle,
        })
    }

    pub fn fill<T>(&self, data: &[T]) -> crate::Result<()> {
        unsafe {
            if let Some(ptr) = self.allocation.mapped_ptr() {
                std::ptr::copy_nonoverlapping(data.as_ptr(), ptr.cast().as_ptr(), data.len());
                Ok(())
            } else {
                Err(Error::string("Buffer memory not mapped"))
            }
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { self.device.destroy_buffer(self.handle, None) };
        let _ = self
            .allocator
            .lock()
            .unwrap()
            .free(std::mem::take(&mut self.allocation));
        neptune_core::log::warn!("Buffer Drop");
    }
}
