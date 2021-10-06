use crate::buffer::Buffer;
use crate::command_buffer::CommandBuffer;
use crate::image::Image;
use crate::{BufferId, ImageId};
use ash::vk;

pub enum ResourceAccess {
    ReadBuffer(BufferId),
    WriteBuffer(BufferId),
    ReadImage(ImageId),
    WriteImage(ImageId),
}

pub trait RenderTask {
    fn get_resources(&self) -> Vec<ResourceAccess>;
    fn build_command(&self, frame_index: u32, command_buffer: &mut CommandBuffer);
}
