mod buffer;
mod id_pool;
mod interface;
mod pipeline;
mod render_graph;
mod resource;
mod texture;
pub mod vulkan;

pub use crate::render_graph::{
    ColorAttachment, DepthStencilAttachment, RasterPassBuilder, RenderGraphBuilder,
};
use std::rc::Rc;

use crate::pipeline::PipelineState;
use crate::render_graph::TextureId;
use crate::vulkan::ShaderModule;
pub use buffer::BufferDescription;
pub use buffer::BufferUsages;
pub use resource::Resource;
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

pub fn render_graph_test(render_graph: &mut RenderGraphBuilder) {
    let (swapchain_id, swapchain_size) = render_graph.get_swapchain_image();
    let swapchain_size = TextureDimensions::D2(swapchain_size[0], swapchain_size[1]);

    let some_buffer = render_graph.create_buffer(BufferDescription {
        size: 16,
        usage: BufferUsages::STORAGE,
        memory_type: MemoryType::GpuOnly,
    });

    let some_texture = render_graph.create_texture(TextureDescription {
        format: TextureFormat::Rgba8Unorm,
        size: swapchain_size,
        usage: TextureUsages::COLOR_ATTACHMENT,
        memory_type: MemoryType::GpuOnly,
    });

    let some_depth_texture = render_graph.create_texture(TextureDescription {
        format: TextureFormat::D32Float,
        size: swapchain_size,
        usage: TextureUsages::DEPTH_STENCIL_ATTACHMENT,
        memory_type: MemoryType::GpuOnly,
    });

    let mut raster_pass = RasterPassBuilder::new("Test");
    raster_pass.attachments(
        &[
            ColorAttachment {
                id: swapchain_id,
                clear: Some([0.5, 0.25, 0.125, 0.0]),
            },
            ColorAttachment {
                id: some_texture,
                clear: Some([0.0; 4]),
            },
        ],
        Some(DepthStencilAttachment {
            id: some_depth_texture,
            clear: Some((1.0, 0)),
        }),
    );
    raster_pass.vertex_buffer(some_buffer);
    render_graph.add_raster_pass(raster_pass);
}

pub fn render_triangle_test(
    render_graph: &mut RenderGraphBuilder,
    render_target: TextureId,
    vertex_module: Rc<ShaderModule>,
    fragment_module: Rc<ShaderModule>,
) {
    let vertex_buffer = render_graph.create_buffer(BufferDescription {
        size: 16,
        usage: BufferUsages::VERTEX | BufferUsages::TRANSFER_DST,
        memory_type: MemoryType::GpuOnly,
    });

    let mut raster_pass = RasterPassBuilder::new("Test");
    raster_pass.attachments(
        &[ColorAttachment {
            id: render_target,
            clear: Some([0.5, 0.25, 0.125, 0.0]),
        }],
        None,
    );
    raster_pass.pipeline(
        vertex_module,
        Some(fragment_module),
        Vec::new(),
        PipelineState::default(),
        |command_buffer| command_buffer.draw(3, 0, 1, 0),
    );
    render_graph.add_raster_pass(raster_pass);
}
