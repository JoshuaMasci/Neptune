use ash::vk;
use gpu_allocator::vulkan;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(PartialEq, Debug)]
pub struct BufferDescription {
    pub size: usize,
    pub usage: vk::BufferUsageFlags,
    pub memory_location: gpu_allocator::MemoryLocation,
}

pub struct Buffer {
    device: Rc<ash::Device>,
    device_allocator: Rc<RefCell<vulkan::Allocator>>,
    pub allocation: gpu_allocator::vulkan::Allocation,
    pub buffer: vk::Buffer,
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub memory_location: gpu_allocator::MemoryLocation,
}

impl Buffer {
    pub(crate) fn new(
        device: Rc<ash::Device>,
        device_allocator: Rc<RefCell<vulkan::Allocator>>,
        create_info: &vk::BufferCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> Self {
        let buffer =
            unsafe { device.create_buffer(create_info, None) }.expect("Failed to create buffer");
        let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let allocation = device_allocator
            .borrow_mut()
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
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
            device,
            device_allocator,
            allocation,
            buffer,
            size: create_info.size,
            usage: create_info.usage,
            memory_location,
        }
    }

    pub(crate) fn fill<T: Copy>(&mut self, data: &[T]) {
        let buffer_ptr = self
            .allocation
            .mapped_ptr()
            .expect("Failed to map buffer memory")
            .as_ptr();

        let mut align =
            unsafe { ash::util::Align::new(buffer_ptr, std::mem::align_of::<T>() as _, self.size) };

        align.copy_from_slice(data);
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.device_allocator
            .borrow_mut()
            .free(self.allocation.clone())
            .expect("Failed to free buffer memory");
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
        }
    }
}
