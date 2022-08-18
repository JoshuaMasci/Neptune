use neptune_graphics::{BufferUsage, DeviceTrait};

pub(crate) struct SceneLayer {
    vertex_buffer: neptune_graphics::Buffer,
    index_buffer: neptune_graphics::Buffer,
    index_count: u32,
}

impl SceneLayer {
    pub(crate) fn new(device: &mut neptune_graphics::Device) -> Self {
        let vertex_data = [
            0.0, -0.5, 0.0, 1.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 1.0, 0.0, -0.5, 0.5, 0.0, 0.0, 0.0,
            1.0,
        ];
        let index_data = [0u32, 1u32, 2u32];

        let vertex_buffer = device
            .create_static_buffer(
                BufferUsage::VERTEX | BufferUsage::TRANSFER_WRITE,
                bytemuck::bytes_of(&vertex_data),
            )
            .unwrap();
        let index_buffer = device
            .create_static_buffer(
                BufferUsage::VERTEX | BufferUsage::TRANSFER_WRITE,
                bytemuck::bytes_of(&index_data),
            )
            .unwrap();

        Self {
            vertex_buffer,
            index_buffer,
            index_count: index_data.len() as u32,
        }
    }

    pub(crate) fn build_render_graph(
        &mut self,
        render_graph_builder: &mut neptune_graphics::RenderGraphBuilder,
    ) {
        let _ = self;
        let _ = render_graph_builder;
    }
}
