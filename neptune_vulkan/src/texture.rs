use crate::resource_manager::ResourceManager;
use crate::{AshDevice, Error};
use ash::vk;
use bitflags::bitflags;
use std::sync::{Arc, Mutex};

bitflags! {
    pub struct TextureUsage: u32 {
        const ATTACHMENT = 1 << 0;
    }
}

bitflags! {
    pub struct TextureBindingType: u32 {
        const SAMPLED = 1 << 0;
        const STORAGE = 1 << 0;
    }
}

pub struct Texture {
    pub(crate) texture: AshTexture,
    pub(crate) resource_manager: Arc<Mutex<ResourceManager>>,
}

impl Drop for Texture {
    fn drop(&mut self) {
        self.resource_manager
            .lock()
            .unwrap()
            .destroy_texture(std::mem::take(&mut self.texture));
    }
}

#[derive(Default, Debug)]
pub struct AshTexture {
    pub handle: vk::Image,
    pub allocation: gpu_allocator::vulkan::Allocation,
}

impl AshTexture {
    pub(crate) fn create_texture(
        device: &Arc<AshDevice>,
        allocator: &Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
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

        Ok(Self { allocation, handle })
    }

    pub(crate) fn destroy_texture(
        &mut self,
        device: &Arc<AshDevice>,
        allocator: &Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    ) {
        unsafe { device.destroy_image(self.handle, None) };
        let _ = allocator
            .lock()
            .unwrap()
            .free(std::mem::take(&mut self.allocation));
        trace!("Destroy Image");
    }

    fn unsafe_clone(&self) -> Self {
        let mut allocation: gpu_allocator::vulkan::Allocation = Default::default();

        // Using unsafe because Allocation doesn't impl Clone despite the fact it is just raw data.
        // This is likely because they don't want multiple of the mapped pointers around, which shouldn't cause a problem
        unsafe {
            std::ptr::copy_nonoverlapping(&self.allocation, &mut allocation, 1);
        }

        Self {
            handle: self.handle,
            allocation,
        }
    }
}
