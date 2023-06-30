mod debug_utils;
mod device;
mod instance;
mod render_graph;
mod swapchain;

//Public Types
pub use crate::device::AshDevice;
pub use crate::instance::AppInfo;
pub use crate::instance::AshInstance;
pub use crate::render_graph::{
    BasicRenderGraphExecutor, BufferAccess, BufferResource, ColorAttachment,
    DepthStencilAttachment, Framebuffer, ImageAccess, ImageResource, RenderGraph,
    RenderGraphResources, RenderPass,
};
pub use crate::swapchain::{AshSwapchain, AshSwapchainSettings, SwapchainManager};

pub use ash::vk;

use log::warn;
use slotmap::{new_key_type, SlotMap};
use std::collections::HashMap;

new_key_type! {
   pub struct BufferKey;
   pub struct ImageKey;
}

pub struct AshBuffer {
    handle: vk::Buffer,
}
pub struct AshBufferResource {
    buffer: AshBuffer,
}

#[derive(Debug, Clone)]
pub struct AshBufferResourceAccess {
    queue: vk::Queue,
    write: bool, //TODO: calculate this from stage+access?
    stage: vk::PipelineStageFlags2,
    access: vk::AccessFlags2,
}

impl AshBufferResourceAccess {
    pub fn conflicts_with(&self, other: &Self) -> bool {
        // accesses conflict if the queues differ or either access is a write
        (self.queue != other.queue) || (self.write || other.write)
    }

    pub fn merge_reads(&self, other: &Self) -> Self {
        assert_eq!(
            self.queue, other.queue,
            "Cannot merge resource accesses with differing queues"
        );
        assert!(
            !self.write,
            "Cannot merge resource accesses if self is a write"
        );
        assert!(
            !other.write,
            "Cannot merge resource accesses if other is a write"
        );

        Self {
            queue: self.queue,
            write: false,
            stage: self.stage | other.stage,
            access: self.access | other.access,
        }
    }
}

pub struct AshImage {
    handle: vk::Image,
    view: vk::ImageView,
    extend: vk::Extent2D,
}
pub struct AshImageResource {
    image: AshImage,
}

#[derive(Debug, Clone)]
pub struct AshImageResourceAccess {
    queue: vk::Queue,
    write: bool, //TODO: calculate this from stage+access?
    stage: vk::PipelineStageFlags2,
    access: vk::AccessFlags2,
    layout: vk::ImageLayout,
}

impl AshImageResourceAccess {
    pub fn conflicts_with(&self, other: &Self) -> bool {
        // accesses conflict if the queues/layout differ or either access is a write
        (self.queue != other.queue) || (self.layout != other.layout) || (self.write || other.write)
    }

    pub fn merge_reads(&self, other: &Self) -> Self {
        assert_eq!(
            self.queue, other.queue,
            "Cannot merge resource accesses with differing queues"
        );
        assert_eq!(
            self.layout, other.layout,
            "Cannot merge resource accesses with differing layouts"
        );
        assert!(
            !self.write,
            "Cannot merge resource accesses if self is a write"
        );
        assert!(
            !other.write,
            "Cannot merge resource accesses if other is a write"
        );

        Self {
            queue: self.queue,
            write: false,
            stage: self.stage | other.stage,
            access: self.access | other.access,
            layout: self.layout,
        }
    }
}

#[derive(Default)]
pub struct ResourceFrame {
    freed_buffers: Vec<BufferKey>,
    buffer_accesses: HashMap<BufferKey, AshBufferResourceAccess>,
    freed_images: Vec<ImageKey>,
    image_accesses: HashMap<ImageKey, AshImageResourceAccess>,
}

pub struct PersistentResourceManager {
    current_frame_index: usize,
    frames: Vec<ResourceFrame>,

    buffers: SlotMap<BufferKey, AshBufferResource>,
    images: SlotMap<ImageKey, AshImageResource>,
}

impl PersistentResourceManager {
    pub fn new(frame_count: usize) -> Self {
        Self {
            current_frame_index: 0,
            frames: (0..frame_count).map(|_| ResourceFrame::default()).collect(),
            buffers: SlotMap::with_key(),
            images: SlotMap::with_key(),
        }
    }

    pub fn flush_frame(&mut self) {
        self.current_frame_index = (self.current_frame_index + 1) % self.frames.len();
        let frame = &mut self.frames[self.current_frame_index];

        for key in frame.freed_buffers.drain(..) {
            if self.buffers.remove(key).is_some() {
                warn!("BufferKey({:?}) was invalid on deletion", key);
            }
        }
        frame.buffer_accesses.clear();

        for key in frame.freed_images.drain(..) {
            if self.images.remove(key).is_some() {
                warn!("ImageKey({:?}) was invalid on deletion", key);
            }
        }
        frame.image_accesses.clear();
    }

    pub fn add_buffer(&mut self, buffer: AshBuffer) -> BufferKey {
        self.buffers.insert(AshBufferResource { buffer })
    }

    pub fn get_buffer(&self, key: BufferKey) -> Option<&AshBuffer> {
        self.buffers.get(key).map(|resource| &resource.buffer)
    }

    pub fn get_last_buffer_access(&self, key: BufferKey) -> Option<AshBufferResourceAccess> {
        let mut last_access: Option<AshBufferResourceAccess> = None;

        if self.buffers.contains_key(key) {
            loop_backward_with_wraparound(&self.frames, self.current_frame_index, |frame| {
                if let Some(access) = frame.buffer_accesses.get(&key) {
                    if let Some(last_access) = last_access.as_mut() {
                        if last_access.conflicts_with(access) {
                            return LoopState::Exit;
                        } else {
                            *last_access = last_access.merge_reads(access);
                        }
                    } else {
                        last_access = Some(access.clone());
                    }
                }

                LoopState::Continue
            });
        }

        last_access
    }

    pub fn set_last_buffer_access(&mut self, key: BufferKey, access: AshBufferResourceAccess) {
        let length = self.frames.len();
        let frame = &mut self.frames[self.current_frame_index % length];
        if frame.buffer_accesses.insert(key, access).is_some() {
            warn!(
                "BufferKey({:?}) access was already written this frame, this shouldn't happen",
                key
            );
        }
    }

    pub fn add_image(&mut self, image: AshImage) -> ImageKey {
        self.images.insert(AshImageResource { image })
    }

    pub fn get_image(&self, key: ImageKey) -> Option<&AshImage> {
        self.images.get(key).map(|resource| &resource.image)
    }

    pub fn get_last_image_access(&self, key: ImageKey) -> Option<AshImageResourceAccess> {
        let mut last_access: Option<AshImageResourceAccess> = None;

        if self.images.contains_key(key) {
            loop_backward_with_wraparound(&self.frames, self.current_frame_index, |frame| {
                if let Some(access) = frame.image_accesses.get(&key) {
                    if let Some(last_access) = last_access.as_mut() {
                        if last_access.conflicts_with(access) {
                            return LoopState::Exit;
                        } else {
                            *last_access = last_access.merge_reads(access);
                        }
                    } else {
                        last_access = Some(access.clone());
                    }
                }

                LoopState::Continue
            });
        }

        last_access
    }

    pub fn set_last_image_access(&mut self, key: ImageKey, access: AshImageResourceAccess) {
        let length = self.frames.len();
        let frame = &mut self.frames[self.current_frame_index % length];
        if frame.image_accesses.insert(key, access).is_some() {
            warn!(
                "ImageKey({:?}) access was already written this frame, this shouldn't happen",
                key
            );
        }
    }
}

pub struct TransientResourceManager {}

#[derive(PartialEq)]
enum LoopState {
    Continue,
    Exit,
}

fn loop_backward_with_wraparound<T, F>(vector: &[T], start_index: usize, mut callback: F)
where
    F: FnMut(&T) -> LoopState,
{
    let length = vector.len();
    let mut index = start_index % length;
    for _ in 0..length {
        if callback(&vector[index]) == LoopState::Exit {
            return;
        }
        if index == 0 {
            index = length - 1;
        } else {
            index -= 1;
        }
    }
}
