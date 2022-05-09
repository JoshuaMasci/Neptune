use crate::interface::{RasterApi, RasterCommandBuffer};
use crate::render_graph::BufferHandle;
use crate::IndexSize;
use ash::vk;
use std::rc::Rc;

pub(crate) struct VulkanRasterCommandBuffer {
    device: Rc<ash::Device>,
    command_buffer: vk::CommandBuffer,
}

impl RasterCommandBuffer for VulkanRasterCommandBuffer {
    fn bind_vertex_buffers(&mut self, buffers: &[(BufferHandle, u32)]) {
        todo!()
    }

    fn bind_index_buffer(&mut self, buffer: BufferHandle, offset: u32, index_type: IndexSize) {
        todo!()
    }

    fn draw(
        &mut self,
        vertex_count: u32,
        first_vertex: u32,
        instance_count: u32,
        instance_offset: u32,
    ) {
        todo!()
    }

    fn draw_indexed(
        &mut self,
        index_count: u32,
        first_index: u32,
        vertex_offset: i32,
        instance_count: u32,
        instance_offset: u32,
    ) {
        todo!()
    }
}

pub(crate) struct VulkanRasterApi {
    device: Rc<ash::Device>,
    command_buffer: vk::CommandBuffer,
}

impl RasterApi for VulkanRasterApi {
    fn bind_pipeline<F>(&mut self, some: u32, raster_fn: F)
    where
        F: FnOnce(&mut dyn RasterCommandBuffer),
    {
        let mut raster_command_buffer = VulkanRasterCommandBuffer {
            device: self.device.clone(),
            command_buffer: self.command_buffer.clone(),
        };

        raster_fn(&mut raster_command_buffer);
    }
}
