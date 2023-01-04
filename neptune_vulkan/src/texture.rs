use crate::{AshDevice, Error};
use ash::vk;
use bitflags::bitflags;
use std::sync::{Arc, Mutex};

bitflags! {
    pub struct TextureUsage: u32 {
        const ATTACHMENT = 1 << 0;
        const SAMPLED = 1 << 1;
        const STORAGE = 1 << 2;
    }
}

fn is_color_format(format: vk::Format) -> bool {
    match format {
        vk::Format::D16_UNORM
        | vk::Format::D16_UNORM_S8_UINT
        | vk::Format::D24_UNORM_S8_UINT
        | vk::Format::X8_D24_UNORM_PACK32
        | vk::Format::D32_SFLOAT
        | vk::Format::D32_SFLOAT_S8_UINT => false,
        _ => true,
    }
}

pub(crate) fn get_vk_texture_2d_create_info(
    usage: TextureUsage,
    format: vk::Format,
    size: [u32; 2],
) -> vk::ImageCreateInfo {
    let mut vk_usage = vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST;

    let is_color_format = is_color_format(format);
    if usage.contains(TextureUsage::ATTACHMENT) {
        vk_usage |= match is_color_format {
            true => vk::ImageUsageFlags::COLOR_ATTACHMENT,
            false => vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        };
    }

    if usage.contains(TextureUsage::SAMPLED) {
        vk_usage |= vk::ImageUsageFlags::SAMPLED;
    }

    if usage.contains(TextureUsage::STORAGE) {
        vk_usage |= vk::ImageUsageFlags::STORAGE;
    }

    vk::ImageCreateInfo::builder()
        .format(format)
        .image_type(vk::ImageType::TYPE_2D)
        .usage(vk_usage)
        .extent(vk::Extent3D {
            width: size[0],
            height: size[1],
            depth: 1,
        })
        .array_layers(1)
        .mip_levels(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .build()
}

pub struct AshImage {
    device: Arc<AshDevice>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    pub handle: vk::Image,
    pub allocation: gpu_allocator::vulkan::Allocation,
    pub view: vk::ImageView,
}

impl AshImage {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
        usage: TextureUsage,
        format: vk::Format,
        size: [u32; 2],
        memory_location: gpu_allocator::MemoryLocation,
    ) -> crate::Result<Self> {
        let create_info = get_vk_texture_2d_create_info(usage, format, size);
        let handle = match unsafe { device.create_image(&create_info, None) } {
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

        let view_create_info = vk::ImageViewCreateInfo::builder()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .image(handle)
            .components(vk::ComponentMapping::default())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: if is_color_format(format) {
                    vk::ImageAspectFlags::COLOR
                } else {
                    vk::ImageAspectFlags::DEPTH
                },
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build();

        let view = match unsafe { device.create_image_view(&view_create_info, None) } {
            Ok(view) => view,
            Err(e) => {
                unsafe { device.destroy_image(handle, None) };
                let _ = allocator.lock().unwrap().free(allocation);
                return Err(Error::VkError(e));
            }
        };

        Ok(Self {
            device,
            allocator,
            allocation,
            handle,
            view,
        })
    }
}

impl Drop for AshImage {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.view, None);
            self.device.destroy_image(self.handle, None)
        };
        let _ = self
            .allocator
            .lock()
            .unwrap()
            .free(std::mem::take(&mut self.allocation));
        trace!("Destroy Texture");
    }
}
