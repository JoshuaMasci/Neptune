use crate::resource::ResourceDrop;
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
    device: Rc<ash::Device>,
    allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,

    pub description: BufferDescription,
    pub allocation: gpu_allocator::vulkan::Allocation,
    pub handle: vk::Buffer,
    pub binding: Option<u32>,
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
                    .usage(description.usage),
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
                location: description.memory_location,
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
        let allocation = std::mem::take(&mut self.allocation);

        self.allocator
            .get_mut()
            .free(allocation)
            .expect("Failed to free buffer memory");
        unsafe {
            self.device.destroy_buffer(self.handle, None);
        }
    }
}

impl ResourceDrop for Buffer {
    fn drop_resource(deleter: &mut ResourceDeleter, resource: Self) {
        deleter.free_buffer(resource);
    }
}
