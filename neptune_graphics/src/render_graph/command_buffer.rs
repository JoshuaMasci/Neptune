use crate::buffer::BufferHandle;
use crate::IndexSize;

pub struct RasterCommandBuffer {
    pub(crate) commands: Vec<RasterCommand>,
}

impl RasterCommandBuffer {
    pub(crate) fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn bind_vertex_buffers(&mut self, vertex_buffers: &[(BufferHandle, u32)]) {
        self.commands
            .push(RasterCommand::BindVertexBuffers(vertex_buffers.to_vec()));
    }

    pub fn bind_index_buffer(&mut self, index_buffer: BufferHandle, offset: u32, size: IndexSize) {
        self.commands
            .push(RasterCommand::BindIndexBuffer(index_buffer, offset, size));
    }

    pub fn draw(
        &mut self,
        vertex_range: std::ops::Range<u32>,
        instance_range: std::ops::Range<u32>,
    ) {
        self.commands.push(RasterCommand::Draw {
            vertex_range,
            instance_range,
        });
    }

    pub fn draw_indexed(
        &mut self,
        index_range: std::ops::Range<u32>,
        base_vertex: i32,
        instance_range: std::ops::Range<u32>,
    ) {
        self.commands.push(RasterCommand::DrawIndexed {
            index_range,
            base_vertex,
            instance_range,
        });
    }
}

pub enum RasterCommand {
    BindVertexBuffers(Vec<(BufferHandle, u32)>),
    BindIndexBuffer(BufferHandle, u32, IndexSize),
    //TODO: PushConstants(VK) / RootConstants?(DX12)
    Draw {
        vertex_range: std::ops::Range<u32>,
        instance_range: std::ops::Range<u32>,
    },
    DrawIndexed {
        index_range: std::ops::Range<u32>,
        base_vertex: i32,
        instance_range: std::ops::Range<u32>,
    },
    //TODO: Indirect draw
}
