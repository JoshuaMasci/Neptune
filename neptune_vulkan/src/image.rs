use crate::{AshDevice, Error};
use ash::vk;
use std::sync::{Arc, Mutex};

pub struct Image {
    device: Arc<AshDevice>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,

    pub handle: vk::Image,
    pub allocation: gpu_allocator::vulkan::Allocation,
}

impl Image {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
        create_info: &vk::ImageCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> crate::Result<Self> {
        let handle = match unsafe { device.create_image(create_info, None) } {
            Ok(handle) => handle,
            Err(e) => return Err(Error::VkError(e)),
        };

        let requirements = unsafe { device.get_image_memory_requirements(handle) };

        let allocation =
            match allocator
                .lock()
                .unwrap()
                .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                    name: "Image Allocation",
                    requirements,
                    location: memory_location,
                    linear: true,
                }) {
                Ok(allocation) => allocation,
                Err(e) => {
                    unsafe { device.destroy_image(handle, None) };
                    return Err(Error::GpuAllocError(e));
                }
            };

        if let Err(e) =
            unsafe { device.bind_image_memory(handle, allocation.memory(), allocation.offset()) }
        {
            unsafe { device.destroy_image(handle, None) };
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
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe { self.device.destroy_image(self.handle, None) };
        let _ = self
            .allocator
            .lock()
            .unwrap()
            .free(std::mem::take(&mut self.allocation));
        neptune_core::log::warn!("Image Drop");
    }
}
