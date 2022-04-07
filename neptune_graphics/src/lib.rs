pub mod buffer;
mod internal;
mod texture;

use std::ops::Range;
use std::sync::Arc;

pub struct Surface {}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum MemoryType {
    GpuOnly,
    CpuToGpu,
    GpuToCpu,
}

pub struct Device {
    device: Arc<dyn crate::internal::DeviceImpl>,
}

impl Device {
    fn create_buffer(&mut self, desription: &crate::buffer::BufferDescription) {}

    fn create_texture_1d(&mut self, size: u32, description: &crate::texture::TextureDescription) {}
    fn create_texture_2d(
        &mut self,
        size: [u32; 2],
        description: &crate::texture::TextureDescription,
    ) {
    }
    fn create_texture_3d(
        &mut self,
        size: [u32; 3],
        description: &crate::texture::TextureDescription,
    ) {
    }
    fn create_texture_cube(
        &mut self,
        size: [u32; 2],
        description: &crate::texture::TextureDescription,
    ) {
    }

    //TODO: add render graph
    fn render(&mut self) {}
}

pub trait CommandBuffer {
    fn bind_compute_pipeline(&mut self);
    fn dispatch_compute(&mut self);

    fn bind_graphics_pipeline(&mut self);
    fn bind_index_buffers(&mut self, buffer: &crate::buffer::Buffer);

    fn set_viewport(&mut self);
    fn set_scissor(&mut self);

    fn draw_indexed(&mut self, indices: Range<u32>, instances: Range<u32>);
}
