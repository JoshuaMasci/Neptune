use crate::render_graph::{BufferHandle, TextureHandle};

#[derive(Copy, Clone)]
pub enum IndexType {
    U16,
    U32,
}

pub trait CommandBuffer {
    fn blit_image(&mut self, src: TextureHandle, dst: TextureHandle);
    fn copy_buffer(
        &mut self,
        src: BufferHandle,
        src_offset: u32,
        dst: BufferHandle,
        dst_offset: u32,
        size: u32,
    );
    fn copy_image(&mut self, src: TextureHandle, dst: TextureHandle);
    fn copy_buffer_to_image(&mut self, src: BufferHandle, dst: TextureHandle);
    fn copy_image_to_buffer(&mut self, src: TextureHandle, dst: BufferHandle);

    fn push_data<T>(&mut self, offset: u32, data: &[T]);

    fn bind_compute_pipeline(&mut self);
    fn dispatch(&mut self, size: [u32; 3]);
    fn dispatch_base(&mut self, offset: [u32; 3], size: [u32; 3]);

    fn bind_graphics_pipeline(&mut self);
    fn bind_vertex_buffers(&mut self, buffers: &[(BufferHandle, u32)]);
    fn bind_index_buffer(&mut self, buffer: BufferHandle, offset: u32, index_type: IndexType);

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
