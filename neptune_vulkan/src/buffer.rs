use crate::descriptor_set::{DescriptorBinding, GpuBindingIndex};
use crate::device::AshDevice;
use crate::VulkanError;
use ash::vk;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BufferDescription {
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub location: gpu_allocator::MemoryLocation,
}

pub struct Buffer {
    pub device: Arc<AshDevice>,
    pub handle: vk::Buffer,
    pub allocation: gpu_allocator::vulkan::Allocation,
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub location: gpu_allocator::MemoryLocation,
    pub storage_binding: Option<DescriptorBinding>,
}

impl Buffer {
    pub fn new(
        device: Arc<AshDevice>,
        name: &str,
        description: &BufferDescription,
    ) -> Result<Self, VulkanError> {
        let handle = unsafe {
            device.core.create_buffer(
                &vk::BufferCreateInfo::builder()
                    .size(description.size)
                    .usage(description.usage)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )
        }?;

        if let Some(debug_util) = &device.instance.debug_utils {
            debug_util.set_object_name(device.core.handle(), handle, name);
        }

        let requirements = unsafe { device.core.get_buffer_memory_requirements(handle) };

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
                device.core.destroy_buffer(handle, None);
                return Err(VulkanError::from(err));
            },
        };

        if let Err(err) = unsafe {
            device
                .core
                .bind_buffer_memory(handle, allocation.memory(), allocation.offset())
        } {
            unsafe {
                device.core.destroy_buffer(handle, None);
            };
            let _ = device.allocator.lock().unwrap().free(allocation);
            return Err(VulkanError::from(err));
        }

        Ok(Self {
            device,
            handle,
            allocation,
            size: description.size,
            usage: description.usage,
            location: description.location,
            storage_binding: None,
        })
    }

    pub fn get_copy(&self) -> AshBuffer {
        AshBuffer {
            handle: self.handle,
            size: self.size,
            usage: self.usage,
            location: self.location,
            storage_binding: self.storage_binding.as_ref().map(|binding| binding.index()),
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device.core.destroy_buffer(self.handle, None);
        };
        let _ = self
            .device
            .allocator
            .lock()
            .unwrap()
            .free(std::mem::take(&mut self.allocation));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AshBuffer {
    pub handle: vk::Buffer,
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub location: gpu_allocator::MemoryLocation,
    pub storage_binding: Option<GpuBindingIndex>,
}
