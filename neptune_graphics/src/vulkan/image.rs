use ash::vk;
use std::sync::{Arc, Mutex};

use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum AshImageCreateError {
    #[error("Vk error: {0}")]
    VkError(ash::vk::Result),

    #[error("Gpu alloc error: {0}")]
    GpuAllocError(gpu_allocator::AllocationError),
}

pub(crate) struct AshImage {
    device: Arc<ash::Device>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    pub(crate) handle: vk::Image,
    pub(crate) view_handle: vk::ImageView,
    pub(crate) allocation: gpu_allocator::vulkan::Allocation,
}

impl AshImage {
    pub(crate) fn new(
        device: Arc<ash::Device>,
        allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
        name: &str,
        create_info: &vk::ImageCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> Result<Self, AshImageCreateError> {
        let handle = match unsafe { device.create_image(create_info, None) } {
            Ok(handle) => handle,
            Err(e) => return Err(AshImageCreateError::VkError(e)),
        };

        let requirements = unsafe { device.get_image_memory_requirements(handle) };

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
                    unsafe { device.destroy_image(handle, None) };
                    return Err(AshImageCreateError::GpuAllocError(e));
                }
            };

        if let Err(e) =
            unsafe { device.bind_image_memory(handle, allocation.memory(), allocation.offset()) }
        {
            unsafe { device.destroy_image(handle, None) };
            let _ = allocator.lock().unwrap().free(allocation);
            return Err(AshImageCreateError::VkError(e));
        }

        Ok(Self {
            device,
            allocator,
            handle,
            view_handle: vk::ImageView::null(),
            allocation,
        })
    }

    pub(crate) fn create_view(
        &mut self,
        view_create_info: &vk::ImageViewCreateInfo,
    ) -> ash::prelude::VkResult<()> {
        self.view_handle = unsafe { self.device.create_image_view(view_create_info, None)? };
        Ok(())
    }
}

impl Drop for AshImage {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.view_handle, None);
            self.device.destroy_image(self.handle, None);
        }
        let _ = self
            .allocator
            .lock()
            .unwrap()
            .free(std::mem::take(&mut self.allocation));
    }
}
