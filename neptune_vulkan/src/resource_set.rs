use crate::buffer::Buffer;
use crate::upload_queue::UploadQueue;
use crate::{BufferKey, BufferSetKey, VulkanError};

#[derive(Default)]
pub struct ResourceSetManager {}

impl ResourceSetManager {
    pub fn create_buffer_set(
        &mut self,
        name: &str,
        count: usize,
    ) -> Result<BufferSetKey, VulkanError> {
        Ok(BufferSetKey::default())
    }
    pub fn destroy_buffer_set(&mut self, key: BufferSetKey) {}
    pub fn update_buffer_set(&mut self, key: BufferSetKey, index: usize, value: Option<BufferKey>) {
    }

    pub fn write_transfers(&mut self, transfers: &mut UploadQueue) {}
}

pub struct BufferSet {
    pub name: String,
    pub handles: Vec<Option<BufferKey>>,

    pub gpu_buffer_index: usize,
    pub gpu_buffers: Vec<Buffer>,
}
