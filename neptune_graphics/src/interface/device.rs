use crate::interface::RasterCommandBuffer;
use crate::render_graph::BufferId;
use crate::{BufferDescription, IndexSize};

pub trait Device {
    type Buffer;
    type Texture;
    type CommandBuffer;

    fn create_buffer(&mut self, description: BufferDescription, name: &'static str)
        -> Self::Buffer;
    fn render(
        &mut self,
        build_render_graph: impl FnOnce(
            &mut RenderGraphBuilder<Self::Buffer, Self::Texture, Self::CommandBuffer>,
        ),
    );
}

enum BufferResourceDescription<'a, Buffer> {
    New(BufferDescription),
    Import(&'a Buffer),
}

enum TextureResourceDescription<'a, Texture> {
    New(BufferDescription),
    Import(&'a Texture),
}

pub struct RenderGraphBuilder<'a, Buffer, Texture, CommandBuffer: RasterCommandBuffer> {
    buffers: Vec<BufferResourceDescription<'a, Buffer>>,
    textures: Vec<TextureResourceDescription<'a, Texture>>,

    temp: Vec<Box<dyn FnOnce(CommandBuffer)>>,
}

struct TestCommandBuffer();
impl RasterCommandBuffer for TestCommandBuffer {
    fn bind_vertex_buffers(&mut self, buffers: &[(BufferId, u32)]) {}
    fn bind_index_buffer(&mut self, buffer: BufferId, offset: u32, index_type: IndexSize) {}
    fn draw(
        &mut self,
        vertex_count: u32,
        first_vertex: u32,
        instance_count: u32,
        instance_offset: u32,
    ) {
    }
    fn draw_indexed(
        &mut self,
        index_count: u32,
        first_index: u32,
        vertex_offset: i32,
        instance_count: u32,
        instance_offset: u32,
    ) {
    }
}

pub fn test_func() {
    let buffer: u32 = 125;

    {
        let mut rgb: RenderGraphBuilder<u32, u32, TestCommandBuffer> = RenderGraphBuilder {
            buffers: vec![],
            textures: vec![],
            temp: vec![],
        };

        rgb.buffers.push(BufferResourceDescription::Import(&buffer));
    }
}
