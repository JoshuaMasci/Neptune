use crate::buffer::AshBuffer;
use crate::texture::AshTexture;
use crate::AshDevice;
use std::sync::{Arc, Mutex};

pub(crate) struct ResourceManager {
    device: Arc<AshDevice>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,

    current_frame: usize,
}

impl ResourceManager {
    pub(crate) fn new(
        frames_in_flight_count: usize,
        device: Arc<AshDevice>,
        allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    ) -> Self {
        Self {
            device,
            allocator,
            current_frame: 0,
        }
    }

    pub(crate) fn set_frame(&mut self, frame_index: usize) {
        self.current_frame = frame_index;
    }

    pub(crate) fn destroy_buffer(&mut self, mut buffer: AshBuffer) {
        //Drop Immediately for now
        buffer.destroy_buffer(&self.device, &self.allocator);
    }

    pub(crate) fn destroy_texture(&mut self, mut texture: AshTexture) {
        //Drop Immediately for now
        texture.destroy_texture(&self.device, &self.allocator);
    }
}

impl Drop for ResourceManager {
    fn drop(&mut self) {}
}
