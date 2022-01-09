use crate::render_backend::RenderDevice;
use ash::vk;
use gpu_allocator::vulkan;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(PartialEq, Debug)]
pub struct ImageDescription {
    pub format: vk::Format,
    pub size: [u32; 2],
    pub usage: vk::ImageUsageFlags,
    pub memory_location: gpu_allocator::MemoryLocation,
}

pub struct Image {
    device: Rc<ash::Device>,
    device_allocator: Rc<RefCell<vulkan::Allocator>>,
    pub allocation: gpu_allocator::vulkan::Allocation,
    pub image: vk::Image,
    pub size: vk::Extent3D,
    pub usage: vk::ImageUsageFlags,
    pub memory_location: gpu_allocator::MemoryLocation,
    pub image_view: Option<vk::ImageView>,
}

impl Image {
    pub(crate) fn from_existing_no_drop(
        device: Rc<ash::Device>,
        device_allocator: Rc<RefCell<vulkan::Allocator>>,
        image: vk::Image,
        size: vk::Extent2D,
    ) -> Self {
        Self {
            device,
            device_allocator,
            allocation: Default::default(),
            image,
            size: vk::Extent3D {
                width: size.width,
                height: size.height,
                depth: 1,
            },
            usage: Default::default(),
            memory_location: gpu_allocator::MemoryLocation::Unknown,
            image_view: None,
        }
    }

    pub(crate) fn new_2d(
        device: &RenderDevice,
        description: &ImageDescription,
        view: bool,
    ) -> Self {
        let mut new_self = Self::new(
            device.base.clone(),
            device.allocator.clone(),
            vk::ImageCreateInfo::builder()
                .flags(vk::ImageCreateFlags::empty())
                .usage(description.usage)
                .format(description.format)
                .extent(vk::Extent3D {
                    width: description.size[0],
                    height: description.size[1],
                    depth: 1,
                })
                .samples(vk::SampleCountFlags::TYPE_1)
                .mip_levels(1)
                .array_layers(1)
                .image_type(vk::ImageType::TYPE_2D)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .build(),
            description.memory_location,
        );

        let aspect_mask: vk::ImageAspectFlags = if description
            .usage
            .contains(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
        {
            vk::ImageAspectFlags::DEPTH
        } else {
            vk::ImageAspectFlags::COLOR
        };

        if view {
            new_self.image_view = Some(
                unsafe {
                    device.base.create_image_view(
                        &vk::ImageViewCreateInfo::builder()
                            .format(description.format)
                            .image(new_self.image)
                            .view_type(vk::ImageViewType::TYPE_2D)
                            .components(vk::ComponentMapping {
                                r: vk::ComponentSwizzle::IDENTITY,
                                g: vk::ComponentSwizzle::IDENTITY,
                                b: vk::ComponentSwizzle::IDENTITY,
                                a: vk::ComponentSwizzle::IDENTITY,
                            })
                            .subresource_range(vk::ImageSubresourceRange {
                                aspect_mask,
                                base_mip_level: 0,
                                level_count: 1,
                                base_array_layer: 0,
                                layer_count: 1,
                            }),
                        None,
                    )
                }
                .expect("Failed to create image view"),
            );
        }

        new_self
    }

    pub(crate) fn new(
        device: Rc<ash::Device>,
        device_allocator: Rc<RefCell<vulkan::Allocator>>,
        create_info: vk::ImageCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> Self {
        let image =
            unsafe { device.create_image(&create_info, None) }.expect("Failed to create image");

        let requirements = unsafe { device.get_image_memory_requirements(image) };

        let allocation = device_allocator
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
            device_allocator,
            allocation,
            image,
            size: create_info.extent,
            usage: create_info.usage,
            memory_location,
            image_view: None,
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        //TEMP workaround
        if self.memory_location == gpu_allocator::MemoryLocation::Unknown {
            return;
        }

        self.device_allocator
            .borrow_mut()
            .free(self.allocation.clone())
            .expect("Failed to free image memory");
        unsafe {
            self.device.destroy_image(self.image, None);
        }
    }
}
