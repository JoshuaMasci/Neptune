mod buffer;
mod id_pool;
mod interface;
mod pipeline;
mod render_graph;
mod resource;
mod texture;
pub mod vulkan;

use crate::render_graph::{DepthStencilAttachment, RasterPassBuilder};
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
    let mut render_graph = crate::render_graph::RenderGraphBuilder::new();

    let some_buffer = render_graph.create_buffer(16, MemoryType::GpuOnly);
    let some_texture = render_graph.create_texture(
        TextureFormat::Rgba8Unorm,
        TextureDimensions::D2(128, 128),
        MemoryType::GpuOnly,
    );

    let some_depth_texture = render_graph.create_texture(
        TextureFormat::D32Float,
        TextureDimensions::D2(1920, 1080),
        MemoryType::GpuOnly,
    );

    render_graph.add_raster_pass(
        RasterPassBuilder::new("Test")
            .attachments(
                &[],
                Some(DepthStencilAttachment {
                    id: some_depth_texture,
                    clear: Some([1.0, 0.0]),
                }),
            )
            .vertex_buffer(some_buffer)
            .raster_fn(move |_, _| println!("Rendering: {}", some_depth_texture)),
    );
}
