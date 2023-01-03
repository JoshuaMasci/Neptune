use crate::resource_manager::BufferHandle;
use std::collections::HashMap;

pub enum Queue {
    Primary,
    AsyncCompute,
    AsyncTransfer,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum BufferAccess {
    IndexBufferRead,
    VertexBufferRead,
    TransferRead,
    TransferWrite,
    UniformRead,
    StorageRead,
    StorageWrite,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum TextureAccess {
    AttachmentWrite,
    TransferRead,
    TransferWrite,
    SampledRead,
    StorageRead,
    StorageWrite,
}

pub struct RenderPassBuilder {
    name: String,
    queue: Queue,
    buffer_access: HashMap<BufferHandle, BufferAccess>,
    texture_access: HashMap<TextureAccess, TextureAccess>,
}
