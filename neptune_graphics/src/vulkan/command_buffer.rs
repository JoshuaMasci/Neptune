use crate::render_graph::BufferId;
use crate::vulkan::graph::{BufferStorage, TextureStorage};
use crate::IndexSize;
use ash::vk;
use std::rc::Rc;

pub struct VulkanRasterCommandBuffer<'a> {
    device: Rc<ash::Device>,
    command_buffer: vk::CommandBuffer,
    buffers: &'a [BufferStorage],
    textures: &'a [TextureStorage],
}

impl<'a> VulkanRasterCommandBuffer<'a> {
    pub(crate) fn new(
        device: Rc<ash::Device>,
        command_buffer: vk::CommandBuffer,
        buffers: &'a [BufferStorage],
        textures: &'a [TextureStorage],
    ) -> Self {
        Self {
            device,
            command_buffer,
            buffers,
            textures,
        }
    }

    //TODO: push descriptor interface

    pub fn bind_vertex_buffers(&mut self, buffer: BufferId, offset: u32) {
        unsafe {
            self.device.cmd_bind_vertex_buffers(
                self.command_buffer,
                0,
                &[self.buffers[buffer].get_handle()],
                &[offset as vk::DeviceSize],
            );
        }
    }

    pub fn bind_index_buffer(&mut self, buffer: BufferId, offset: u32, index_type: IndexSize) {
        unsafe {
            self.device.cmd_bind_index_buffer(
                self.command_buffer,
                self.buffers[buffer].get_handle(),
                offset as vk::DeviceSize,
                index_type.to_vk(),
            );
        }
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
        index_offset: u32,
        vertex_offset: i32,
        instance_count: u32,
        instance_offset: u32,
    ) {
        unsafe {
            self.device.cmd_draw_indexed(
                self.command_buffer,
                index_count,
                instance_count,
                index_offset,
                vertex_offset,
                instance_offset,
            );
        }
    }
}
