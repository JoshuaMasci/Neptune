use crate::MemoryType;
use bitflags::bitflags;

bitflags! {
    pub struct BufferUsages: u32 {
        const TRANSFER_SRC = 1 << 0;
        const TRANSFER_DST = 1 << 1;
        const STORAGE = 1 << 2;
        const INDEX = 1 << 3;
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct BufferDescription {
    pub size: usize,
    pub usage: BufferUsages,
    pub memory_type: MemoryType,
}
