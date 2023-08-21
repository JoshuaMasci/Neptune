use crate::device::AshDevice;
use crate::VulkanError;
use ash::vk;

#[derive(Debug, Clone)]
pub struct BufferDescription {
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub memory_location: gpu_allocator::MemoryLocation,
}

pub struct Buffer {
    pub handle: vk::Buffer,
    pub allocation: gpu_allocator::vulkan::Allocation,
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub location: gpu_allocator::MemoryLocation,
}

impl Buffer {
    pub fn new(
        device: &AshDevice,
        create_info: &vk::BufferCreateInfo,
        location: gpu_allocator::MemoryLocation,
    ) -> Result<Self, VulkanError> {
        let handle = unsafe { device.core.create_buffer(create_info, None) }?;

        let requirements = unsafe { device.core.get_buffer_memory_requirements(handle) };

        let allocation = device.allocator.lock().unwrap().allocate(
            &gpu_allocator::vulkan::AllocationCreateDesc {
                name: "Buffer Allocation",
                requirements,
                location,
                linear: true,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            },
        );

        if allocation.is_err() {
            unsafe {
                device.core.destroy_buffer(handle, None);
            }
        }
        let allocation = allocation?;

        let bind_result = unsafe {
            device
                .core
                .bind_buffer_memory(handle, allocation.memory(), allocation.offset())
        };

        let new_self = Self {
            handle,
            allocation,
            size: create_info.size,
            usage: create_info.usage,
            location,
        };

        if let Err(err) = bind_result {
            new_self.delete(&device);
            Err(err.into())
        } else {
            Ok(new_self)
        }
    }

    pub fn new_desc(
        device: &AshDevice,
        buffer_description: &BufferDescription,
    ) -> Result<Self, VulkanError> {
        Self::new(
            device,
            &vk::BufferCreateInfo::builder()
                .size(buffer_description.size)
                .usage(buffer_description.usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE),
            buffer_description.memory_location,
        )
    }

    pub fn delete(self, device: &AshDevice) {
        unsafe {
            device.core.destroy_buffer(self.handle, None);
        };

        let _ = device.allocator.lock().unwrap().free(self.allocation);
    }
}
