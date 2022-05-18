mod graph;
mod render_graph;
mod render_pass;

use crate::resource::Resource;
pub use render_graph::RenderGraphBuilder;
pub use render_pass::ColorAttachment;
pub use render_pass::ComputePassBuilder;
pub use render_pass::DepthStencilAttachment;
pub use render_pass::RasterPassBuilder;
use std::rc::Rc;

pub type PassId = usize;
pub type BufferId = usize;
pub type TextureId = usize;

//TODO: use abstract types
pub type RasterFn = dyn FnOnce(Rc<ash::Device>, ash::vk::CommandBuffer);

pub type ImportedBuffer = Rc<Resource<crate::vulkan::Buffer>>;
pub type ImportedTexture = Rc<Resource<crate::vulkan::Texture>>;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum BufferAccess {
    None,

    IndexBufferRead,
    VertexBufferRead,

    TransferRead,
    TransferWrite,

    ShaderRead,
    ShaderWrite,
}

impl BufferAccess {
    pub fn is_write(&self) -> bool {
        match self {
            BufferAccess::None
            | BufferAccess::IndexBufferRead
            | BufferAccess::VertexBufferRead
            | BufferAccess::TransferRead
            | BufferAccess::ShaderRead => false,
            BufferAccess::TransferWrite | BufferAccess::ShaderWrite => true,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum TextureAccess {
    None,

    ColorAttachmentWrite,
    DepthStencilAttachmentWrite,

    TransferRead,
    TransferWrite,

    ShaderSampledRead,
    ShaderStorageRead,
    ShaderStorageWrite,
}

impl TextureAccess {
    pub fn is_write(&self) -> bool {
        match self {
            TextureAccess::None
            | TextureAccess::TransferRead
            | TextureAccess::ShaderSampledRead
            | TextureAccess::ShaderStorageRead
            | TextureAccess::TransferWrite => false,
            TextureAccess::ShaderStorageWrite
            | TextureAccess::ColorAttachmentWrite
            | TextureAccess::DepthStencilAttachmentWrite => true,
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) struct ResourceAccess<T: Copy> {
    pub(crate) pass_id: PassId,
    pub(crate) access: T,
}

pub(crate) enum ResourceAccessType<T: Copy> {
    Write(ResourceAccess<T>),
    Reads(Vec<ResourceAccess<T>>),
}
