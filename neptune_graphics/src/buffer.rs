use bitflags::bitflags;
use std::fmt::{Debug, Formatter};

//Buffer API
bitflags! {
    pub struct BufferUsage: u32 {
        const TRANSFER_READ = 1 << 0;
        const TRANSFER_WRITE = 1 << 1; //TODO: delete this? Almost all buffers will require this, otherwise it can't be written to from the cpu
        const VERTEX = 1 << 2;
        const INDEX = 1 << 3;
        const UNIFORM = 1 << 4;
        const STORAGE = 1 << 5;
        const INDIRECT  = 1 << 6;
    }
}

pub type BufferHandle = u32;

pub enum BufferGraphResource {
    External(BufferHandle),
    Transient(usize), //TODO: What should this be?
}

pub trait BufferResource {
    fn get_graph_resource(&self) -> BufferGraphResource;
}

pub struct Buffer {
    handle: BufferHandle,
    freed_list: std::sync::Mutex<Vec<BufferHandle>>,
}

impl Buffer {
    pub fn new_temp(handle: BufferHandle) -> Self {
        Self {
            handle,
            freed_list: std::sync::Mutex::new(vec![]),
        }
    }
}

impl BufferResource for Buffer {
    fn get_graph_resource(&self) -> BufferGraphResource {
        BufferGraphResource::External(self.handle)
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if let Ok(mut freed_list) = self.freed_list.lock() {
            freed_list.push(self.handle);
        }
    }
}

impl Debug for Buffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Buffer")
            .field("handle", &self.handle)
            .finish()
    }
}
