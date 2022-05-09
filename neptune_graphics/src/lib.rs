mod buffer;
mod id_pool;
mod interface;
mod pipeline;
mod render_graph;
mod resource;
mod texture;
pub mod vulkan;

use crate::render_graph::{
    AttachmentLoadOp, BufferAccessType, RenderPassBuilder, TextureAccessType,
};
pub use buffer::BufferDescription;
pub use buffer::BufferUsages;
pub use texture::TextureDescription;
pub use texture::TextureDimensions;
pub use texture::TextureFormat;
pub use texture::TextureUsages;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum IndexSize {
    U16,
    U32,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum MemoryType {
    GpuOnly,
    CpuToGpu,
    GpuToCpu,
}

impl MemoryType {
    pub fn to_gpu_alloc(self) -> gpu_allocator::MemoryLocation {
        match self {
            MemoryType::GpuOnly => gpu_allocator::MemoryLocation::GpuOnly,
            MemoryType::CpuToGpu => gpu_allocator::MemoryLocation::CpuToGpu,
            MemoryType::GpuToCpu => gpu_allocator::MemoryLocation::GpuToCpu,
        }
    }
}

pub fn render_graph_test() {
    let mut render_graph = crate::render_graph::RenderGraph::default();

    let some_buffer = render_graph.create_buffer(BufferDescription {
        size: 16,
        usage: BufferUsages::STORAGE | BufferUsages::TRANSFER_SRC | BufferUsages::TRANSFER_DST,
        memory_type: MemoryType::GpuOnly,
    });

    let some_texture = render_graph.create_texture(TextureDescription {
        format: TextureFormat::Rgba8Unorm,
        size: TextureDimensions::D2(128, 128),
        usage: TextureUsages::SAMPLED | TextureUsages::TRANSFER_DST,
        memory_type: MemoryType::GpuOnly,
    });

    let some_depth_texture = render_graph.create_texture(TextureDescription {
        format: TextureFormat::D32Float,
        size: TextureDimensions::D2(1920, 1080),
        usage: TextureUsages::DEPTH_STENCIL_ATTACHMENT,
        memory_type: MemoryType::GpuOnly,
    });

    render_graph.add_render_pass(
        RenderPassBuilder::new("Pass1")
            .buffer(some_buffer, BufferAccessType::ShaderWrite)
            .texture(some_texture, TextureAccessType::ShaderStorageWrite),
    );

    render_graph.add_render_pass(
        RenderPassBuilder::new("Pass2").buffer(some_buffer, BufferAccessType::VertexBuffer),
    );

    render_graph.add_render_pass(
        RenderPassBuilder::new("Pass3")
            .buffer(some_buffer, BufferAccessType::VertexBuffer)
            .raster(
                &[],
                Some((some_depth_texture, AttachmentLoadOp::Clear([1.0; 4]))),
            )
            .render(move || {
                println!("render_fn: {:?}", some_depth_texture.get());
            }),
    );
}
