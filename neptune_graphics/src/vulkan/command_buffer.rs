use crate::render_graph::BufferId;
use crate::vulkan::graph::{BufferStorage, TextureStorage};
use crate::{IndexSize, TextureId};
use ash::vk;
use std::rc::Rc;

pub struct VulkanRasterCommandBuffer<'a> {
    device: Rc<ash::Device>,
    layout: vk::PipelineLayout,
    command_buffer: vk::CommandBuffer,
    buffers: &'a [BufferStorage],
    textures: &'a [TextureStorage],
}

impl<'a> VulkanRasterCommandBuffer<'a> {
    pub(crate) fn new(
        device: Rc<ash::Device>,
        layout: vk::PipelineLayout,
        command_buffer: vk::CommandBuffer,
        buffers: &'a [BufferStorage],
        textures: &'a [TextureStorage],
    ) -> Self {
        Self {
            device,
            layout,
            command_buffer,
            buffers,
            textures,
        }
    }

    //TODO: push descriptor interface
    pub fn push_texture(&mut self, offset: u32, texture: TextureId) {
        let binding = match &self.textures[texture] {
            TextureStorage::Unused => panic!(),
            TextureStorage::Swapchain(_, _, _, _) => panic!(),
            TextureStorage::Temporary(texture) => texture.sampled_binding.as_ref().unwrap().index,
            TextureStorage::Imported(texture) => texture.sampled_binding.as_ref().unwrap().index,
        };
        unsafe {
            self.device.cmd_push_constants(
                self.command_buffer,
                self.layout,
                vk::ShaderStageFlags::ALL,
                offset,
                &binding.to_ne_bytes(),
            );
        }
    }

    pub fn push_floats(&mut self, offset: u32, data: &[f32]) {
        //TODO: TEMP conversion
        let mut bytes: Vec<u8> = vec![];
        for float in data.iter() {
            for byte in float.to_ne_bytes() {
                bytes.push(byte);
            }
        }

        unsafe {
            self.device.cmd_push_constants(
                self.command_buffer,
                self.layout,
                vk::ShaderStageFlags::ALL,
                offset,
                &bytes,
            );
        }
    }

    pub fn bind_vertex_buffers(&self, buffer: BufferId, offset: u32) {
        unsafe {
            self.device.cmd_bind_vertex_buffers(
                self.command_buffer,
                0,
                &[self.buffers[buffer].get_handle()],
                &[offset as vk::DeviceSize],
            );
        }
    }

    pub fn bind_index_buffer(&self, buffer: BufferId, offset: u32, index_type: IndexSize) {
        unsafe {
            self.device.cmd_bind_index_buffer(
                self.command_buffer,
                self.buffers[buffer].get_handle(),
                offset as vk::DeviceSize,
                index_type.to_vk(),
            );
        }
    }

    pub fn set_scissor(&self, offset: [i32; 2], extent: [u32; 2]) {
        unsafe {
            self.device.cmd_set_scissor(
                self.command_buffer,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D {
                        x: offset[0],
                        y: offset[1],
                    },
                    extent: vk::Extent2D {
                        width: extent[0],
                        height: extent[1],
                    },
                }],
            );
        }
    }

    pub fn draw(
        &self,
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
        &self,
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
