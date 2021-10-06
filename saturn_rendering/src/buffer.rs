use ash::vk;
use gpu_allocator::vulkan;

pub struct Buffer {
    pub allocation: gpu_allocator::vulkan::Allocation,
    pub buffer: vk::Buffer,
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub memory_location: gpu_allocator::MemoryLocation,
}

impl Buffer {
    pub(crate) fn new(
        device: &ash::Device,
        allocator: &mut vulkan::Allocator,
        create_info: &vk::BufferCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> Self {
        let buffer =
            unsafe { device.create_buffer(create_info, None) }.expect("Failed to create buffer");
        let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let allocation = allocator
            .allocate(&vulkan::AllocationCreateDesc {
                name: "Buffer Allocation",
                requirements,
                location: memory_location,
                linear: true,
            })
            .expect("Failed to allocate buffer memory");

        unsafe {
            device
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .expect("Failed to bind buffer memory");
        }

        Self {
            allocation,
            buffer,
            size: create_info.size,
            usage: create_info.usage,
            memory_location,
        }
    }

    //Do not use drop as that requires storing device and allocation which is not needed
    pub(crate) fn destroy(&mut self, device: &ash::Device, allocator: &mut vulkan::Allocator) {
        allocator
            .free(self.allocation.clone())
            .expect("Failed to free buffer memory");
        unsafe {
            device.destroy_buffer(self.buffer, None);
        }
    }
}
