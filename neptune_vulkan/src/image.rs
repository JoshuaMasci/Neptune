use crate::device::AshDevice;
use crate::render_graph::TransientImageDesc;
use ash::vk;

pub fn vk_format_get_aspect_flags(format: vk::Format) -> vk::ImageAspectFlags {
    match format {
        vk::Format::D16_UNORM | vk::Format::D32_SFLOAT | vk::Format::X8_D24_UNORM_PACK32 => {
            vk::ImageAspectFlags::DEPTH
        }
        vk::Format::S8_UINT => vk::ImageAspectFlags::STENCIL,
        vk::Format::D32_SFLOAT_S8_UINT | vk::Format::D24_UNORM_S8_UINT => {
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
        }
        _ => vk::ImageAspectFlags::COLOR,
    }
}

#[derive(Debug, Clone)]
pub struct ImageDescription2D {
    pub size: [u32; 2],
    pub format: vk::Format,
    pub usage: vk::ImageUsageFlags,
    pub mip_levels: u32,
    pub memory_location: gpu_allocator::MemoryLocation,
}

impl ImageDescription2D {
    pub(crate) fn from_transient(resolved_size: [u32; 2], desc: &TransientImageDesc) -> Self {
        Self {
            size: resolved_size,
            format: desc.format,
            usage: desc.usage,
            mip_levels: desc.mip_levels,
            memory_location: desc.memory_location,
        }
    }
}

pub struct Image {
    pub handle: vk::Image,
    pub view: vk::ImageView,
    pub allocation: gpu_allocator::vulkan::Allocation,
    pub extend: vk::Extent2D,
    pub format: vk::Format,
    pub usage: vk::ImageUsageFlags,
    pub location: gpu_allocator::MemoryLocation,
}

impl Image {
    pub fn new_2d(device: &AshDevice, desc: ImageDescription2D) -> Self {
        let create_info = vk::ImageCreateInfo::builder()
            .format(desc.format)
            .extent(vk::Extent3D {
                width: desc.size[0],
                height: desc.size[1],
                depth: 1,
            })
            .usage(desc.usage)
            .array_layers(1)
            .mip_levels(desc.mip_levels)
            .samples(vk::SampleCountFlags::TYPE_1)
            .image_type(vk::ImageType::TYPE_2D);

        let handle =
            unsafe { device.core.create_image(&create_info, None) }.expect("TODO: return error");

        let requirements = unsafe { device.core.get_image_memory_requirements(handle) };

        let allocation = device
            .allocator
            .lock()
            .unwrap()
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "Image Allocation",
                requirements,
                location: desc.memory_location,
                linear: true,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .expect("TODO: return error");

        unsafe {
            device
                .core
                .bind_image_memory(handle, allocation.memory(), allocation.offset())
                .expect("TODO: return error");
        }

        let mut view_create_info = vk::ImageViewCreateInfo::builder()
            .image(handle)
            .format(desc.format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk_format_get_aspect_flags(desc.format),
                base_mip_level: 0,
                level_count: desc.mip_levels,
                base_array_layer: 0,
                layer_count: 1,
            })
            .view_type(vk::ImageViewType::TYPE_2D);

        let view = unsafe { device.core.create_image_view(&view_create_info, None) }
            .expect("TODO: return error");

        Self {
            handle,
            view,
            allocation,
            extend: vk::Extent2D {
                width: create_info.extent.width,
                height: create_info.extent.height,
            },
            format: create_info.format,
            usage: create_info.usage,
            location: desc.memory_location,
        }
    }

    pub fn delete(self, device: &AshDevice) {
        unsafe {
            device.core.destroy_image_view(self.view, None);
            device.core.destroy_image(self.handle, None);
        };

        let _ = device.allocator.lock().unwrap().free(self.allocation);
    }
}
