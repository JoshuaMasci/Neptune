use crate::render_backend::RenderDevice;
use crate::resource::ResourceDrop;
use crate::resource_deleter::ResourceDeleter;
use ash::vk;
use ash::vk::ImageView;
use gpu_allocator::vulkan;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct ImageDescription {
    pub format: vk::Format,
    pub size: [u32; 2],
    pub usage: vk::ImageUsageFlags,
    pub memory_location: gpu_allocator::MemoryLocation,
}

pub struct Image {
    device: Rc<ash::Device>,
    allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,

    pub description: ImageDescription,
    pub memory: gpu_allocator::vulkan::Allocation,
    pub handle: vk::Image,
    pub view: Option<vk::ImageView>,
}

impl Image {
    pub(crate) fn new(
        device: Rc<ash::Device>,
        allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,
        description: ImageDescription,
    ) -> Self {
        let handle = unsafe {
            device.create_image(
                &vk::ImageCreateInfo::builder()
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
                None,
            )
        }
        .expect("Failed to create image");

        let requirements = unsafe { device.get_image_memory_requirements(handle) };

        let memory = allocator
            .borrow_mut()
            .allocate(&vulkan::AllocationCreateDesc {
                name: "Image Allocation",
                requirements,
                location: description.memory_location,
                linear: true,
            })
            .expect("Failed to allocate image memory");

        unsafe {
            device
                .bind_image_memory(handle, memory.memory(), memory.offset())
                .expect("Failed to bind image memory");
        }

        Self {
            device,
            allocator,
            description,
            memory,
            handle,
            view: None,
        }
    }

    // pub(crate) fn from_existing(
    //     description: ImageDescription,
    //     handle: vk::Image,
    //     view: Option<vk::ImageView>,
    // ) -> Self {
    //     Self {
    //         description,
    //         memory: Default::default(),
    //         handle,
    //         view,
    //     }
    // }

    pub(crate) fn create_image_view(&mut self) {
        let aspect_mask: vk::ImageAspectFlags = if self
            .description
            .usage
            .contains(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
        {
            vk::ImageAspectFlags::DEPTH
        } else {
            vk::ImageAspectFlags::COLOR
        };

        self.view = Some(
            unsafe {
                self.device.create_image_view(
                    &vk::ImageViewCreateInfo::builder()
                        .format(self.description.format)
                        .image(self.handle)
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
}

impl Drop for Image {
    fn drop(&mut self) {
        let allocation = std::mem::take(&mut self.memory);

        self.allocator
            .borrow_mut()
            .free(allocation)
            .expect("Failed to free image memory");
        unsafe {
            if let Some(view) = self.view {
                self.device.destroy_image_view(view, None);
            }
            self.device.destroy_image(self.handle, None);
        }
    }
}

impl ResourceDrop for Image {
    fn drop_resource(deleter: &mut ResourceDeleter, resource: Self) {
        deleter.free_image(resource);
    }
}
