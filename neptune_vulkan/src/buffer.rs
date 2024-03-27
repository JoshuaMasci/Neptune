use crate::descriptor_set::{DescriptorBinding, GpuBindingIndex};
use crate::device::AshDevice;
use crate::{BufferWriteError, VulkanError};
use ash::vk;
use bitflags::bitflags;
use std::sync::Arc;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferUsage(u32);
bitflags! {
    impl BufferUsage: u32 {
        const VERTEX = 1 << 0;
        const INDEX = 1 << 1;
        const UNIFORM = 1 << 2;
        const STORAGE = 1 << 3;
        const INDIRECT  = 1 << 4;
        const TRANSFER = 1 << 5;
    }
}

impl BufferUsage {
    pub(crate) fn to_vk(&self) -> vk::BufferUsageFlags {
        //Needed to keep clippy from complaining about contains function
        #[allow(unused)]
        use bitflags::Flags;

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

        if self.contains(BufferUsage::TRANSFER) {
            vk_usage |= vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST;
        }

        vk_usage
    }
}

#[derive(Debug, Clone)]
pub struct BufferDescription {
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub location: gpu_allocator::MemoryLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferDescription2 {
    /// Size in bytes of the allocated buffer
    pub size: usize,

    /// Specifies the intended usage of the buffer
    pub usage: BufferUsage,

    /// Allocates a buffer per frame in flight, allowing better parallelism
    /// Allows cpu writes for the buffer using either mapped memory if platform supports it or a staging buffer
    pub per_frame_memory_mapped: bool,
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
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        location: gpu_allocator::MemoryLocation,
    ) -> Result<Self, VulkanError> {
        let handle = unsafe {
            device.core.create_buffer(
                &vk::BufferCreateInfo::builder()
                    .size(size)
                    .usage(usage)
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
                location,
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
            size,
            usage,
            location,
            storage_binding: None,
        })
    }

    pub fn is_mapped(&self) -> bool {
        self.allocation.mapped_slice().is_some()
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
