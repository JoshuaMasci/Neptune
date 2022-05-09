use crate::render_graph::BufferHandle;
use crate::IndexSize;

pub trait RasterCommandBuffer {
    //TODO: "Object Safe" push constant function
    //fn push_data<T>(&mut self, offset: u32, data: &[T]);

    fn bind_vertex_buffers(&mut self, buffers: &[(BufferHandle, u32)]);
    fn bind_index_buffer(&mut self, buffer: BufferHandle, offset: u32, index_type: IndexSize);

    fn draw(
        &mut self,
        vertex_count: u32,
        first_vertex: u32,
        instance_count: u32,
        instance_offset: u32,
    );
    fn draw_indexed(
        &mut self,
        index_count: u32,
        first_index: u32,
        vertex_offset: i32,
        instance_count: u32,
        instance_offset: u32,
    );
}

pub trait RasterApi {
    fn bind_pipeline<F>(&mut self, some: u32, raster_fn: F)
    where
        F: FnOnce(&mut dyn RasterCommandBuffer);
}
