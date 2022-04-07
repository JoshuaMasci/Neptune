use crate::render_backend::RenderDevice;
use crate::resource_deleter::ResourceDeleter;
use crate::vulkan::BindingType;
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct BufferDescription {
    pub size: usize,
    pub usage: vk::BufferUsageFlags,
    pub memory_location: gpu_allocator::MemoryLocation,
}

pub struct Buffer {
    resource_deleter: Rc<RefCell<ResourceDeleter>>,

    pub description: BufferDescription,
    pub memory: gpu_allocator::vulkan::Allocation,
    pub handle: vk::Buffer,
    pub binding: Option<u32>,
}

impl Buffer {
    pub(crate) fn new(
        device: &RenderDevice,
        resource_deleter: Rc<RefCell<ResourceDeleter>>,
        description: BufferDescription,
    ) -> Self {
        let handle = unsafe {
            device.base.create_buffer(
                &vk::BufferCreateInfo::builder()
                    .size(description.size as vk::DeviceSize)
                    .usage(description.usage),
                None,
            )
        }
        .expect("Failed to create buffer");
        let requirements = unsafe { device.base.get_buffer_memory_requirements(handle) };

        let memory = device
            .allocator
            .borrow_mut()
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "Buffer Allocation",
                requirements,
                location: description.memory_location,
                linear: true,
            })
            .expect("Failed to allocate buffer memory");

        unsafe {
            device
                .base
                .bind_buffer_memory(handle, memory.memory(), memory.offset())
                .expect("Failed to bind buffer memory");
        }

        Self {
            resource_deleter,
            description,
            memory,
            handle,
            binding: None,
        }
    }

    pub(crate) fn fill<T>(&self, data: &[T]) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.memory
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
        let mut resource_deleter = self.resource_deleter.borrow_mut();
        resource_deleter.free_buffer(self.handle, std::mem::take(&mut self.memory));

        if let Some(binding) = self.binding {
            resource_deleter.free_binding(BindingType::StorageBuffer, binding);
        }
    }
}
