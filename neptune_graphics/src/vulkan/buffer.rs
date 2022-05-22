use crate::buffer::{BufferDescription, BufferUsages};
use crate::vulkan::descriptor_set::Binding;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;

impl BufferUsages {
    fn to_vk(&self) -> vk::BufferUsageFlags {
        let mut result = vk::BufferUsageFlags::empty();
        if self.contains(BufferUsages::TRANSFER_SRC) {
            result |= vk::BufferUsageFlags::TRANSFER_SRC;
        }
        if self.contains(BufferUsages::TRANSFER_DST) {
            result |= vk::BufferUsageFlags::TRANSFER_DST;
        }
        if self.contains(BufferUsages::STORAGE) {
            result |= vk::BufferUsageFlags::STORAGE_BUFFER;
        }
        if self.contains(BufferUsages::INDEX) {
            result |= vk::BufferUsageFlags::INDEX_BUFFER;
        }
        result
    }
}

pub struct Buffer {
    device: Rc<ash::Device>,
    allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,

    pub description: BufferDescription,
    pub allocation: gpu_allocator::vulkan::Allocation,
    pub handle: vk::Buffer,
    pub binding: Option<Binding>,
}

impl Buffer {
    pub(crate) fn new(
        device: Rc<ash::Device>,
        allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,
        description: BufferDescription,
    ) -> Self {
        let handle = unsafe {
            device.create_buffer(
                &vk::BufferCreateInfo::builder()
                    .size(description.size as vk::DeviceSize)
                    .usage(description.usage.to_vk()),
                None,
            )
        }
        .expect("Failed to create buffer");
        let requirements = unsafe { device.get_buffer_memory_requirements(handle) };

        let allocation = allocator
            .borrow_mut()
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "Buffer Allocation",
                requirements,
                location: description.memory_type.to_gpu_alloc(),
                linear: true,
            })
            .expect("Failed to allocate buffer memory");

        unsafe {
            device
                .bind_buffer_memory(handle, allocation.memory(), allocation.offset())
                .expect("Failed to bind buffer memory");
        }

        Self {
            device,
            allocator,
            description,
            allocation,
            handle,
            binding: None,
        }
    }

    pub(crate) fn fill<T>(&self, data: &[T]) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.allocation
                    .mapped_ptr()
                    .expect("Failed to map buffer memory")
                    .cast()
                    .as_ptr(),
                data.len(),
            )
        };
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.handle, None);
        }

        let allocation = std::mem::take(&mut self.allocation);
        self.allocator
            .borrow_mut()
            .free(allocation)
            .expect("Failed to free buffer memory");
    }
}
