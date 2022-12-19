use crate::resource_manager::ResourceManager;
use crate::{AshDevice, Error};
use ash::vk;
use bitflags::bitflags;
use std::sync::{Arc, Mutex};

bitflags! {
    pub struct BufferUsage: u32 {
        const VERTEX = 1 << 2;
        const INDEX = 1 << 3;
        const INDIRECT  = 1 << 6;
    }
}

//TODO: Should this be flags?
pub enum BufferBindingType {
    None,
    Uniform,
    Storage,
}

pub(crate) fn get_vk_buffer_create_info(
    usage: BufferUsage,
    binding: BufferBindingType,
    size: u64,
) -> vk::BufferCreateInfo {
    let mut vk_usage = vk::BufferUsageFlags::TRANSFER_SRC
        | vk::BufferUsageFlags::TRANSFER_DST
        | match binding {
            BufferBindingType::None => vk::BufferUsageFlags::empty(),
            BufferBindingType::Uniform => vk::BufferUsageFlags::UNIFORM_BUFFER,
            BufferBindingType::Storage => vk::BufferUsageFlags::STORAGE_BUFFER,
        };

    if usage.contains(BufferUsage::VERTEX) {
        vk_usage |= vk::BufferUsageFlags::VERTEX_BUFFER;
    }

    if usage.contains(BufferUsage::INDEX) {
        vk_usage |= vk::BufferUsageFlags::INDEX_BUFFER;
    }

    if usage.contains(BufferUsage::INDIRECT) {
        vk_usage |= vk::BufferUsageFlags::INDIRECT_BUFFER;
    }

    vk::BufferCreateInfo::builder()
        .usage(vk_usage)
        .size(size)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .build()
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum BufferBinding {
    Uniform(u16),
    Storage(u16),
}

pub struct Buffer {
    pub(crate) buffer: AshBuffer,
    pub(crate) resource_manager: Arc<Mutex<ResourceManager>>,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.resource_manager
            .lock()
            .unwrap()
            .destroy_buffer(std::mem::take(&mut self.buffer));
    }
}

#[derive(Default, Debug)]
pub struct AshBuffer {
    pub(crate) handle: vk::Buffer,
    pub(crate) allocation: gpu_allocator::vulkan::Allocation,
    pub(crate) binding: Option<BufferBinding>,
}

impl AshBuffer {
    pub(crate) fn create_buffer(
        device: &Arc<AshDevice>,
        allocator: &Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
        create_info: &vk::BufferCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> crate::Result<Self> {
        let handle = match unsafe { device.create_buffer(create_info, None) } {
            Ok(handle) => handle,
            Err(e) => return Err(Error::VkError(e)),
        };

        let requirements = unsafe { device.get_buffer_memory_requirements(handle) };

        let allocation =
            match allocator
                .lock()
                .unwrap()
                .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                    name: "Buffer Allocation",
                    requirements,
                    location: memory_location,
                    linear: true,
                }) {
                Ok(allocation) => allocation,
                Err(e) => {
                    unsafe { device.destroy_buffer(handle, None) };
                    return Err(Error::GpuAllocError(e));
                }
            };

        if let Err(e) =
            unsafe { device.bind_buffer_memory(handle, allocation.memory(), allocation.offset()) }
        {
            unsafe { device.destroy_buffer(handle, None) };
            let _ = allocator.lock().unwrap().free(allocation);
            return Err(Error::VkError(e));
        }

        Ok(Self {
            handle,
            allocation,
            binding: None,
        })
    }

    pub(crate) fn destroy_buffer(
        &mut self,
        device: &Arc<AshDevice>,
        allocator: &Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    ) {
        unsafe { device.destroy_buffer(self.handle, None) };
        let _ = allocator
            .lock()
            .unwrap()
            .free(std::mem::take(&mut self.allocation));
        trace!("Destroy Buffer");
    }

    fn unsafe_clone(&self) -> Self {
        let mut allocation: gpu_allocator::vulkan::Allocation = Default::default();

        // Using unsafe because Allocation doesn't impl Clone despite the fact it is just raw data.
        // This is likely because they don't want multiple of the mapped pointers around, which shouldn't cause a problem
        unsafe {
            std::ptr::copy_nonoverlapping(&self.allocation, &mut allocation, 1);
        }

        Self {
            handle: self.handle,
            allocation,
            binding: self.binding,
        }
    }
}
