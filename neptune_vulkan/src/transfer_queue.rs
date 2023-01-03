use crate::resource_manager::{BufferHandle, TextureHandle};
use crate::{AshBuffer, AshDevice};
use ash::vk;
use std::sync::{Arc, Mutex};

pub(crate) struct BufferTransfer {
    src_buffer: AshBuffer,
    src_offset: u32,
    dst_buffer: BufferHandle,
    dst_offset: u32,
    copy_size: u32,
}

pub(crate) struct TextureTransfer {
    src_buffer: AshBuffer,
    src_offset: u32,
    dst_image: TextureHandle,
    dst_subresource: vk::ImageSubresource,
    dst_offset: vk::Offset3D,
    dst_extent: vk::Extent3D,
}

pub(crate) struct TransferQueue {
    device: Arc<AshDevice>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
}
