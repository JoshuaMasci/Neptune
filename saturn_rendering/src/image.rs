use ash::vk;
use gpu_allocator::vulkan;

pub(crate) struct Image {
    pub(crate) allocation: gpu_allocator::vulkan::Allocation,
    pub(crate) image: vk::Image,
    pub(crate) size: vk::Extent3D,
    pub(crate) usage: vk::ImageUsageFlags,
    pub(crate) memory_location: gpu_allocator::MemoryLocation,
    image_view: Option<vk::ImageView>,
}

impl Image {
    pub(crate) fn new(
        device: &ash::Device,
        allocator: &mut vulkan::Allocator,
        create_info: &vk::ImageCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> Self {
        let image =
            unsafe { device.create_image(create_info, None) }.expect("Failed to create image");
        let requirements = unsafe { device.get_image_memory_requirements(image) };

        let allocation = allocator
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
            allocation,
            image,
            size: create_info.extent,
            usage: create_info.usage,
            memory_location,
            image_view: None,
        }
    }

    //Do not use drop as that requires storing device and allocation which is not needed
    pub(crate) fn destroy(&mut self, device: &ash::Device, allocator: &mut vulkan::Allocator) {
        allocator
            .free(self.allocation.clone())
            .expect("Failed to free image memory");
        unsafe {
            device.destroy_image(self.image, None);
        }
    }

    pub(crate) fn create_image_view(&mut self, create_info: &vk::ImageViewCreateInfo) {}

    pub(crate) fn get_image_view(&self) -> vk::ImageView {
        self.image_view
            .expect("Image doesn't contain an image view")
    }
}
