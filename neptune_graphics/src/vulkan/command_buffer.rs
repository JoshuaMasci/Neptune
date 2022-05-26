use crate::render_graph::BufferId;
use crate::IndexSize;
use ash::vk;
use std::rc::Rc;

pub struct VulkanRasterCommandBuffer {
    device: Rc<ash::Device>,
    command_buffer: vk::CommandBuffer,
}

impl VulkanRasterCommandBuffer {
    pub(crate) fn new(device: Rc<ash::Device>, command_buffer: vk::CommandBuffer) -> Self {
        Self {
            device,
            command_buffer,
        }
    }

    //TODO: push descriptor interface

    pub fn bind_vertex_buffers(&mut self, buffers: &[(BufferId, u32)]) {
        todo!()
    }

    pub fn bind_index_buffer(&mut self, buffer: BufferId, offset: u32, index_type: IndexSize) {
        todo!()
    }

    pub fn draw(
        &mut self,
        vertex_count: u32,
        first_vertex: u32,
        instance_count: u32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.cmd_draw(
                self.command_buffer,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            );
        }
    }

    pub fn draw_indexed(
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
