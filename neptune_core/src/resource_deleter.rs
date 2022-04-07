use crate::render_backend::RenderDevice;
use crate::vulkan::{BindingType, DescriptorSet};
use ash::vk;
use gpu_allocator::vulkan;

struct FreedBuffer {
    handle: vk::Buffer,
    allocation: vulkan::Allocation,
}

struct FreedImage {
    handle: vk::Image,
    allocation: vulkan::Allocation,
    views: Vec<vk::ImageView>,
}

struct FreedBinding {
    binding_type: BindingType,
    binding: u32,
}

#[derive(Default)]
pub(crate) struct FreedFrame {
    freed_buffer_list: Vec<FreedBuffer>,
    freed_image_list: Vec<FreedImage>,
    freed_binding_list: Vec<FreedBinding>,
}

pub struct ResourceDeleter {
    current_frame: usize,
    frame_list: Vec<FreedFrame>,
}

impl ResourceDeleter {
    pub(crate) fn new(frame_in_flight_count: usize) -> Self {
        Self {
            current_frame: 0,
            frame_list: (0..frame_in_flight_count)
                .map(|_| FreedFrame::default())
                .collect(),
        }
    }

    pub(crate) fn clear_frame(
        &mut self,
        render_device: &RenderDevice,
        descriptor_set: &mut DescriptorSet,
    ) {
        self.current_frame = (self.current_frame + 1) % self.frame_list.len();
        let mut frame = &mut self.frame_list[self.current_frame];

        for freed_buffer in frame.freed_buffer_list.drain(..) {
            unsafe {
                render_device.base.destroy_buffer(freed_buffer.handle, None);
            }
            render_device
                .allocator
                .borrow_mut()
                .free(freed_buffer.allocation);
        }

        for freed_image in frame.freed_image_list.drain(..) {
            unsafe {
                for view in freed_image.views {
                    render_device.base.destroy_image_view(view, None);
                }
                render_device.base.destroy_image(freed_image.handle, None);
            }
            render_device
                .allocator
                .borrow_mut()
                .free(freed_image.allocation);
        }

        for freed_binding in frame.freed_binding_list.drain(..) {
            descriptor_set.clear_binding(freed_binding.binding_type, freed_binding.binding);
        }
    }

    pub(crate) fn free_buffer(&mut self, handle: vk::Buffer, allocation: vulkan::Allocation) {
        self.frame_list[self.current_frame]
            .freed_buffer_list
            .push(FreedBuffer { handle, allocation });
    }

    pub(crate) fn free_image(
        &mut self,
        handle: vk::Image,
        allocation: vulkan::Allocation,
        views: Vec<vk::ImageView>,
    ) {
        self.frame_list[self.current_frame]
            .freed_image_list
            .push(FreedImage {
                handle,
                allocation,
                views,
            });
    }

    pub(crate) fn free_binding(&mut self, binding_type: BindingType, binding: u32) {
        self.frame_list[self.current_frame]
            .freed_binding_list
            .push(FreedBinding {
                binding_type,
                binding,
            });
    }

    pub(crate) fn clear_all(
        &mut self,
        render_device: &RenderDevice,
        descriptor_set: &mut DescriptorSet,
    ) {
        for _ in 0..self.frame_list.len() {
            self.clear_frame(render_device, descriptor_set);
        }
    }
}
