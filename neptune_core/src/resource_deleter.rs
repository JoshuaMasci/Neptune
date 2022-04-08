use crate::render_backend::RenderDevice;
use crate::vulkan::{BindingType, Buffer, DescriptorSet, Image};
use ash::vk;
use gpu_allocator::vulkan;

struct FreedBinding {
    binding_type: BindingType,
    binding: u32,
}

#[derive(Default)]
pub(crate) struct FreedFrame {
    freed_buffer_list: Vec<Buffer>,
    freed_image_list: Vec<Image>,
    freed_binding_list: Vec<FreedBinding>,
}

pub struct ResourceDeleter {
    current_frame: usize,
    frame_list: Vec<FreedFrame>,
}

impl ResourceDeleter {
    pub(crate) fn new(frame_count: usize) -> Self {
        Self {
            current_frame: 0,
            frame_list: (0..frame_count).map(|_| FreedFrame::default()).collect(),
        }
    }

    pub(crate) fn clear_frame(&mut self, descriptor_set: &mut DescriptorSet) {
        self.current_frame = (self.current_frame + 1) % self.frame_list.len();
        let mut frame = &mut self.frame_list[self.current_frame];

        frame.freed_buffer_list.clear();
        frame.freed_image_list.clear();

        for freed_binding in frame.freed_binding_list.drain(..) {
            descriptor_set.clear_binding(freed_binding.binding_type, freed_binding.binding);
        }
    }

    pub(crate) fn free_buffer(&mut self, buffer: Buffer) {
        self.frame_list[self.current_frame]
            .freed_buffer_list
            .push(buffer);
    }

    pub(crate) fn free_image(&mut self, image: Image) {
        self.frame_list[self.current_frame]
            .freed_image_list
            .push(image);
    }

    pub(crate) fn free_binding(&mut self, binding_type: BindingType, binding: u32) {
        self.frame_list[self.current_frame]
            .freed_binding_list
            .push(FreedBinding {
                binding_type,
                binding,
            });
    }

    pub(crate) fn clear_all(&mut self, descriptor_set: &mut DescriptorSet) {
        for _ in 0..self.frame_list.len() {
            self.clear_frame(descriptor_set);
        }
    }
}
