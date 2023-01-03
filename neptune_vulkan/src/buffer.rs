use crate::{AshDevice, Error};
use ash::vk;
use bitflags::bitflags;
use std::sync::{Arc, Mutex};

bitflags! {
    pub struct BufferUsage: u32 {
        const VERTEX = 1 << 2;
        const INDEX = 1 << 3;
        const UNIFORM = 1 << 4;
        const STORAGE = 1 << 5;
        const INDIRECT  = 1 << 6;
    }
}

impl BufferUsage {
    fn to_vk(&self) -> vk::BufferUsageFlags {
        let mut vk_usage = vk::BufferUsageFlags::empty();

        if self.contains(BufferUsage::VERTEX) {
            vk_usage |= vk::BufferUsageFlags::VERTEX_BUFFER;
        }

        if self.contains(BufferUsage::INDEX) {
            vk_usage |= vk::BufferUsageFlags::INDEX_BUFFER;
        }

        if self.contains(BufferUsage::UNIFORM) {
            vk_usage |= vk::BufferUsageFlags::UNIFORM_BUFFER;
        }

        if self.contains(BufferUsage::STORAGE) {
            vk_usage |= vk::BufferUsageFlags::STORAGE_BUFFER;
        }

        if self.contains(BufferUsage::INDIRECT) {
            vk_usage |= vk::BufferUsageFlags::INDIRECT_BUFFER;
        }

        vk_usage
    }
}

pub(crate) fn get_vk_buffer_create_info(usage: BufferUsage, size: u64) -> vk::BufferCreateInfo {
    vk::BufferCreateInfo::builder()
        .usage(
            vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST | usage.to_vk(),
        )
        .size(size)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .build()
}

pub struct AshBuffer {
    device: Arc<AshDevice>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    pub(crate) handle: vk::Buffer,
    pub(crate) allocation: gpu_allocator::vulkan::Allocation,
}

impl AshBuffer {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
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
            device,
            allocator,
            handle,
            allocation,
        })
    }
}

impl Drop for AshBuffer {
    fn drop(&mut self) {
        unsafe { self.device.destroy_buffer(self.handle, None) };
        let _ = self
            .allocator
            .lock()
            .unwrap()
            .free(std::mem::take(&mut self.allocation));
        trace!("Destroy Buffer");
    }
}
