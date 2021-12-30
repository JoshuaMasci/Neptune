use ash::vk;
use gpu_allocator::vulkan;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Image {
    device: ash::Device,
    allocator: Rc<RefCell<vulkan::Allocator>>,

    pub allocation: gpu_allocator::vulkan::Allocation,
    pub image: vk::Image,
    pub size: vk::Extent3D,
    pub usage: vk::ImageUsageFlags,
    pub memory_location: gpu_allocator::MemoryLocation,
}

impl Image {
    pub(crate) fn new(
        device: ash::Device,
        allocator: Rc<RefCell<vulkan::Allocator>>,
        create_info: &vk::ImageCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> Self {
        let image =
            unsafe { device.create_image(create_info, None) }.expect("Failed to create image");
        let requirements = unsafe { device.get_image_memory_requirements(image) };

        let allocation = allocator
            .borrow_mut()
            .allocate(&vulkan::AllocationCreateDesc {
                name: "Image Allocation",
                requirements,
                location: memory_location,
                linear: true,
            })
            .expect("Failed to allocate image memory");

        unsafe {
            device
                .bind_image_memory(image, allocation.memory(), allocation.offset())
                .expect("Failed to bind image memory");
        }

        Self {
            device,
            allocator,
            allocation,
            image,
            size: create_info.extent,
            usage: create_info.usage,
            memory_location,
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        self.allocator
            .borrow_mut()
            .free(self.allocation.clone())
            .expect("Failed to free image memory");
        unsafe {
            self.device.destroy_image(self.image, None);
        }
    }
}
