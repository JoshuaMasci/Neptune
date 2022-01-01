use ash::vk;
use gpu_allocator::vulkan;
use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::null;
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
    pub(crate) fn new_2d(
        device: ash::Device,
        allocator: Rc<RefCell<vulkan::Allocator>>,
        usage: vk::ImageUsageFlags,
        format: vk::Format,
        size: vk::Extent2D,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> Self {
        // Self::new(
        //     device,
        //     allocator,
        //     &vk::ImageCreateInfo::builder()
        //         .flags(vk::ImageCreateFlags::empty())
        //         .usage(usage)
        //         .format(format)
        //         .extent(vk::Extent3D {
        //             width: size.width,
        //             height: size.height,
        //             depth: 1,
        //         })
        //         .samples(vk::SampleCountFlags::TYPE_1)
        //         .mip_levels(1)
        //         .array_layers(1)
        //         .image_type(vk::ImageType::TYPE_2D)
        //         .initial_layout(vk::ImageLayout::UNDEFINED)
        //         .sharing_mode(vk::SharingMode::EXCLUSIVE)
        //         .queue_family_indices(&[0]), //TODO: not this
        //     memory_location,
        // )
        Self::new(
            device,
            allocator,
            vk::ImageCreateInfo {
                s_type: vk::StructureType::IMAGE_CREATE_INFO,
                p_next: null(),
                flags: vk::ImageCreateFlags::empty(),
                image_type: vk::ImageType::TYPE_2D,
                format,
                extent: vk::Extent3D {
                    width: size.width,
                    height: size.height,
                    depth: 1,
                },
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling: vk::ImageTiling::LINEAR,
                usage,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                queue_family_index_count: 0,
                p_queue_family_indices: null(),
                initial_layout: vk::ImageLayout::UNDEFINED,
            },
            memory_location,
        )
    }

    pub(crate) fn new(
        device: ash::Device,
        allocator: Rc<RefCell<vulkan::Allocator>>,
        create_info: vk::ImageCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> Self {
        let image =
            unsafe { device.create_image(&create_info, None) }.expect("Failed to create image");

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
