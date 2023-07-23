use ash::vk;
use std::sync::Arc;

use crate::vulkan::device::{AshDevice, VulkanCreateError};

pub(crate) struct AshImage {
    device: Arc<AshDevice>,

    pub(crate) handle: vk::Image,
    pub(crate) view_handle: vk::ImageView,
    pub(crate) allocation: gpu_allocator::vulkan::Allocation,
}

impl AshImage {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        name: &str,
        create_info: &vk::ImageCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> Result<Self, VulkanCreateError> {
        let handle = unsafe { device.core.create_image(create_info, None)? };

        let requirements = unsafe { device.core.get_image_memory_requirements(handle) };

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
                unsafe { device.core.destroy_image(handle, None) };
                return Err(VulkanCreateError::GpuAllocError(e));
            }
        };

        if let Err(e) = unsafe {
            device
                .core
                .bind_image_memory(handle, allocation.memory(), allocation.offset())
        } {
            unsafe { device.core.destroy_image(handle, None) };
            let _ = device.allocator.lock().unwrap().free(allocation);
            return Err(VulkanCreateError::VkError(e));
        }

        Ok(Self {
            device,
            handle,
            view_handle: vk::ImageView::null(),
            allocation,
        })
    }

    pub(crate) fn create_view(
        &mut self,
        view_create_info: &vk::ImageViewCreateInfo,
    ) -> ash::prelude::VkResult<()> {
        self.view_handle = unsafe { self.device.core.create_image_view(view_create_info, None)? };
        Ok(())
    }
}

impl Drop for AshImage {
    fn drop(&mut self) {
        unsafe {
            self.device.core.destroy_image_view(self.view_handle, None);
            self.device.core.destroy_image(self.handle, None);
        }
        let _ = self
            .device
            .allocator
            .lock()
            .unwrap()
            .free(std::mem::take(&mut self.allocation));
    }
}
