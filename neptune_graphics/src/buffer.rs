use crate::MemoryType;
use bitflags::bitflags;
use std::sync::Arc;

bitflags! {
    pub struct BufferUsages: u32 {
        const TRANSFER_SRC = 1 << 0;
        const TRANSFER_DST = 1 << 1;
        const STORAGE = 1 << 2;
        const VERTEX = 1 << 3;
        const INDEX = 1 << 4;
    }
}

//TODO: tie this type to DeviceImpl???
pub type BufferId = u32;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct BufferDescription {
    pub name: String,
    pub size: usize,
    pub usage: BufferUsages,
    pub memory_type: MemoryType,
}

pub struct Buffer {
    device: Arc<dyn crate::internal::DeviceImpl>,
    handle: BufferId,
}

impl Buffer {
    pub fn binding(&self) -> u32 {
        0
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.device.drop_buffer(self.handle);
    }
}
