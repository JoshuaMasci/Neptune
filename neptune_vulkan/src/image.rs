use crate::{AshDevice, Error};
use ash::prelude::VkResult;
use ash::vk;
use std::ptr::null;
use std::sync::{Arc, Mutex};

pub struct Image {
    device: Arc<AshDevice>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    create_info: vk::ImageCreateInfo,

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
            create_info: *create_info,
        })
    }

    pub fn get_full_image_subresource_range(&self) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR) // TODO: determine this from format
            .base_array_layer(0)
            .layer_count(self.create_info.array_layers)
            .base_mip_level(0)
            .level_count(self.create_info.mip_levels)
            .build()
    }

    pub fn get_full_image_view_create_info(&self) -> vk::ImageViewCreateInfoBuilder {
        vk::ImageViewCreateInfo::builder()
            .image(self.handle)
            .format(self.create_info.format)
            .view_type(vk::ImageViewType::TYPE_2D) //TODO: determine this from image type
            .subresource_range(self.get_full_image_subresource_range())
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
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
        trace!("Drop Image");
    }
}

pub struct ImageView {
    image: Arc<Image>,
    handle: vk::ImageView,
}

impl ImageView {
    pub(crate) fn new(
        image: Arc<Image>,
        create_info: &vk::ImageViewCreateInfo,
    ) -> crate::Result<Self> {
        let handle = match unsafe { image.device.create_image_view(create_info, None) } {
            Ok(handle) => handle,
            Err(e) => return Err(Error::VkError(e)),
        };

        Ok(Self { image, handle })
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        unsafe { self.image.device.destroy_image_view(self.handle, None) };
        trace!("Drop Image View");
    }
}
