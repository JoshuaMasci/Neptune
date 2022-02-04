use crate::render_backend::RenderDevice;
use ash::vk;

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct BufferDescription {
    pub size: usize,
    pub usage: vk::BufferUsageFlags,
    pub memory_location: gpu_allocator::MemoryLocation,
}

pub struct Buffer {
    device: Option<RenderDevice>,
    pub description: BufferDescription,
    pub memory: gpu_allocator::vulkan::Allocation,
    pub handle: vk::Buffer,
}

impl Buffer {
    pub(crate) fn new(device: &RenderDevice, description: BufferDescription) -> Self {
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
            device: Some(device.clone()),
            description,
            memory,
            handle,
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

    pub(crate) fn clone_no_drop(&self) -> Self {
        Self {
            device: None,
            description: self.description,
            memory: self.memory.clone(),
            handle: self.handle,
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if let Some(device) = &self.device {
            device
                .allocator
                .borrow_mut()
                .free(self.memory.clone())
                .expect("Failed to free buffer memory");
            unsafe {
                device.base.destroy_buffer(self.handle, None);
            }
        }
    }
}
