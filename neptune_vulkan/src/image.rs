use crate::descriptor_set::DescriptorBinding;
use crate::device::AshDevice;
use crate::render_graph::TransientImageDesc;
use crate::sampler::Sampler;
use crate::{SamplerHandle, VulkanError};
use ash::vk;
use std::sync::Arc;

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
    pub location: gpu_allocator::MemoryLocation,
    pub sampler: Option<SamplerHandle>,
}

impl ImageDescription2D {
    pub(crate) fn from_transient(resolved_size: [u32; 2], desc: &TransientImageDesc) -> Self {
        Self {
            size: resolved_size,
            format: desc.format,
            usage: desc.usage,
            mip_levels: desc.mip_levels,
            location: desc.memory_location,
            sampler: None,
        }
    }
}

pub struct Image {
    pub device: Arc<AshDevice>,
    pub handle: vk::Image,
    pub view: vk::ImageView,
    pub allocation: gpu_allocator::vulkan::Allocation,
    pub extend: vk::Extent2D,
    pub format: vk::Format,
    pub usage: vk::ImageUsageFlags,
    pub location: gpu_allocator::MemoryLocation,
    pub storage_binding: Option<DescriptorBinding>,
    pub combined_image_sampler: Option<(Arc<Sampler>, DescriptorBinding)>,
}

impl Image {
    pub fn new_2d(
        device: Arc<AshDevice>,
        name: &str,
        description: &ImageDescription2D,
    ) -> Result<Self, VulkanError> {
        let handle = unsafe {
            device.core.create_image(
                &vk::ImageCreateInfo::builder()
                    .format(description.format)
                    .extent(vk::Extent3D {
                        width: description.size[0],
                        height: description.size[1],
                        depth: 1,
                    })
                    .usage(description.usage)
                    .array_layers(1)
                    .mip_levels(description.mip_levels)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .image_type(vk::ImageType::TYPE_2D),
                None,
            )
        }?;

        if let Some(debug_util) = &device.instance.debug_utils {
            debug_util.set_object_name(device.core.handle(), handle, name);
        }

        let requirements = unsafe { device.core.get_image_memory_requirements(handle) };

        let allocation = match device.allocator.lock().unwrap().allocate(
            &gpu_allocator::vulkan::AllocationCreateDesc {
                name,
                requirements,
                location: description.location,
                linear: true,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            },
        ) {
            Ok(allocation) => allocation,
            Err(err) => unsafe {
                device.core.destroy_image(handle, None);
                return Err(VulkanError::from(err));
            },
        };

        if let Err(err) = unsafe {
            device
                .core
                .bind_image_memory(handle, allocation.memory(), allocation.offset())
        } {
            unsafe {
                device.core.destroy_image(handle, None);
            };
            let _ = device.allocator.lock().unwrap().free(allocation);
            return Err(VulkanError::from(err));
        }

        let view_create_info = vk::ImageViewCreateInfo::builder()
            .image(handle)
            .format(description.format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk_format_get_aspect_flags(description.format),
                base_mip_level: 0,
                level_count: description.mip_levels,
                base_array_layer: 0,
                layer_count: 1,
            })
            .view_type(vk::ImageViewType::TYPE_2D);

        let view = match unsafe { device.core.create_image_view(&view_create_info, None) } {
            Ok(view) => view,
            Err(err) => {
                unsafe {
                    device.core.destroy_image(handle, None);
                };
                let _ = device.allocator.lock().unwrap().free(allocation);
                return Err(VulkanError::from(err));
            }
        };

        Ok(Self {
            device,
            handle,
            view,
            allocation,
            extend: vk::Extent2D {
                width: description.size[0],
                height: description.size[1],
            },
            format: description.format,
            usage: description.usage,
            location: description.location,
            storage_binding: None,
            combined_image_sampler: None,
        })
    }

    pub fn get_storage_binding(&self) -> Option<u32> {
        self.storage_binding.as_ref().map(|binding| binding.index())
    }

    pub fn get_sampled_binding(&self) -> Option<u32> {
        self.combined_image_sampler
            .as_ref()
            .map(|(_, binding)| binding.index())
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.device.core.destroy_image_view(self.view, None);
            self.device.core.destroy_image(self.handle, None);
        };

        let _ = self
            .device
            .allocator
            .lock()
            .unwrap()
            .free(std::mem::take(&mut self.allocation));
    }
}
