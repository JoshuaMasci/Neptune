use crate::{AshDevice, Error};
use ash::vk;
use std::sync::{Arc, Mutex};

pub struct Image {
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    device: Arc<AshDevice>,
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
        let aspect_mask = match self.create_info.format {
            vk::Format::D16_UNORM_S8_UINT
            | vk::Format::D24_UNORM_S8_UINT
            | vk::Format::D32_SFLOAT_S8_UINT => {
                vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
            }
            vk::Format::D16_UNORM | vk::Format::D32_SFLOAT => vk::ImageAspectFlags::DEPTH,
            vk::Format::S8_UINT => vk::ImageAspectFlags::STENCIL,
            _ => vk::ImageAspectFlags::COLOR,
        };

        vk::ImageSubresourceRange::builder()
            .aspect_mask(aspect_mask)
            .base_array_layer(0)
            .layer_count(self.create_info.array_layers)
            .base_mip_level(0)
            .level_count(self.create_info.mip_levels)
            .build()
    }

    pub fn get_full_image_view_create_info(&self) -> vk::ImageViewCreateInfoBuilder {
        //TODO: fix edge cases not covered by this
        let view_type = if self.create_info.image_type == vk::ImageType::TYPE_1D {
            vk::ImageViewType::TYPE_1D
        } else if self.create_info.image_type == vk::ImageType::TYPE_2D {
            if self
                .create_info
                .flags
                .contains(vk::ImageCreateFlags::CUBE_COMPATIBLE)
                && self.create_info.array_layers == 6
            {
                vk::ImageViewType::CUBE
            } else {
                vk::ImageViewType::TYPE_2D
            }
        } else if self.create_info.image_type == vk::ImageType::TYPE_3D {
            vk::ImageViewType::TYPE_3D
        } else {
            unreachable!();
        };

        vk::ImageViewCreateInfo::builder()
            .image(self.handle)
            .format(self.create_info.format)
            .view_type(view_type) //TODO: determine this from image type
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
