use crate::buffer::AshBuffer;
use crate::sampler::AshSampler;
use crate::texture::AshTexture;
use crate::AshDevice;
use ash::vk;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub(crate) struct RangeCycle {
    range: std::ops::Range<usize>,
    index: usize,
}

impl RangeCycle {
    pub(crate) fn new(range: std::ops::Range<usize>) -> Self {
        let index = range.start;
        Self { range, index }
    }

    pub(crate) fn get(&self) -> usize {
        self.index
    }

    pub(crate) fn increment(&mut self) {
        let mut new_index = self.index + 1;
        if new_index == self.range.end {
            new_index = self.range.start;
        }
        self.index = new_index;
    }

    pub(crate) fn get_previous(&self, steps: usize) -> usize {
        let mut last_index = self.index;
        for _ in 0..steps {
            last_index = (if last_index == self.range.start {
                self.range.end
            } else {
                last_index
            } - 1);
        }
        last_index
    }
}

#[derive(Copy, Clone)]
pub(crate) enum BufferAccessType {
    Some,
    Other,
}

#[derive(Copy, Clone)]
pub(crate) enum TextureAccessType {
    Some,
    Other,
}

#[derive(Default)]
struct ResourceFrame {
    buffer_usages: HashMap<vk::Buffer, BufferAccessType>,
    texture_usage: HashMap<vk::Image, TextureAccessType>,
}

impl ResourceFrame {
    fn clear(&mut self) {
        self.buffer_usages.clear();
        self.texture_usage.clear();
    }
}

pub(crate) struct ResourceManager {
    device: Arc<AshDevice>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    frames: Vec<ResourceFrame>,
    current_frame: RangeCycle,
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
            frames: (0..frames_in_flight_count)
                .map(|_| ResourceFrame::default())
                .collect(),
            current_frame: RangeCycle::new(0..frames_in_flight_count),
        }
    }

    pub(crate) fn next_frame(&mut self) {
        self.current_frame.increment();
        self.frames[self.current_frame.get()].clear();
    }

    pub(crate) fn get_last_buffer_usage(&self, buffer: &AshBuffer) -> Option<BufferAccessType> {
        for i in 0..self.frames.len() {
            if let Some(usage) = self.frames[self.current_frame.get_previous(i)]
                .buffer_usages
                .get(&buffer.handle)
            {
                return Some(*usage);
            }
        }
        None
    }

    pub(crate) fn destroy_buffer(&mut self, mut buffer: AshBuffer) {
        //Drop Immediately for now
        buffer.destroy_buffer(&self.device, &self.allocator);
    }

    pub(crate) fn destroy_texture(&mut self, mut texture: AshTexture) {
        //Drop Immediately for now
        texture.destroy_texture(&self.device, &self.allocator);
    }

    pub(crate) fn destroy_sampler(&mut self, sampler: AshSampler) {
        //Drop Immediately for now
        sampler.destroy_sampler(&self.device);
    }
}

impl Drop for ResourceManager {
    fn drop(&mut self) {}
}
