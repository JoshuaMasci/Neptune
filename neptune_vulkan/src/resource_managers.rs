use crate::buffer::{Buffer, BufferDescription};
use crate::device::AshDevice;
use crate::image::{Image, ImageDescription2D};
use crate::render_graph::{TransientImageDesc, TransientImageSize, VkImage};
use crate::swapchain::SwapchainImage;
use crate::{BufferKey, ImageHandle, ImageKey};
use ash::vk;
use log::warn;
use slotmap::SlotMap;
use std::collections::HashMap;
use std::sync::Arc;

pub struct AshBufferResource {
    buffer: Buffer,
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

pub struct AshImageHandle {
    image: Image,
}

#[derive(Debug, Clone)]
pub struct AshImageHandleAccess {
    queue: vk::Queue,
    write: bool, //TODO: calculate this from stage+access?
    stage: vk::PipelineStageFlags2,
    access: vk::AccessFlags2,
    layout: vk::ImageLayout,
}

impl AshImageHandleAccess {
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
    image_accesses: HashMap<ImageKey, AshImageHandleAccess>,
}

pub struct PersistentResourceManager {
    device: Arc<AshDevice>,

    current_frame_index: usize,
    frames: Vec<ResourceFrame>,

    buffers: SlotMap<BufferKey, AshBufferResource>,
    images: SlotMap<ImageKey, AshImageHandle>,
}

impl PersistentResourceManager {
    pub fn new(device: Arc<AshDevice>, frame_count: usize) -> Self {
        Self {
            device,
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

    pub fn add_buffer(&mut self, buffer: Buffer) -> BufferKey {
        self.buffers.insert(AshBufferResource { buffer })
    }

    pub fn get_buffer(&self, key: BufferKey) -> Option<&Buffer> {
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

    pub fn add_image(&mut self, image: Image) -> ImageKey {
        self.images.insert(AshImageHandle { image })
    }

    pub fn get_image(&self, key: ImageKey) -> Option<&Image> {
        self.images.get(key).map(|resource| &resource.image)
    }

    pub fn get_last_image_access(&self, key: ImageKey) -> Option<AshImageHandleAccess> {
        let mut last_access: Option<AshImageHandleAccess> = None;

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

    pub fn set_last_image_access(&mut self, key: ImageKey, access: AshImageHandleAccess) {
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

impl Drop for PersistentResourceManager {
    fn drop(&mut self) {
        for (_key, buffer) in self.buffers.drain() {
            buffer.buffer.delete(&self.device);
        }

        for (_key, image) in self.images.drain() {
            image.image.delete(&self.device);
        }
    }
}

pub struct TransientResourceManager {
    device: Arc<AshDevice>,

    transient_buffers: Vec<Buffer>,
    transient_images: Vec<Image>,
}

impl TransientResourceManager {
    pub fn new(device: Arc<AshDevice>) -> Self {
        Self {
            device,
            transient_buffers: vec![],
            transient_images: vec![],
        }
    }

    pub(crate) fn resolve_buffers(
        &mut self,
        transient_image_descriptions: &[BufferDescription],
    ) -> &Vec<Buffer> {
        for buffer_description in transient_image_descriptions {
            self.transient_buffers.push(
                Buffer::new_desc(&self.device, buffer_description).expect("TODO: replace this"),
            )
        }

        &self.transient_buffers
    }

    pub(crate) fn resolve_images(
        &mut self,
        persistent: &mut PersistentResourceManager,
        swapchain_images: &[(vk::SwapchainKHR, SwapchainImage)],
        transient_image_descriptions: &[TransientImageDesc],
    ) -> Vec<VkImage> {
        let mut transient_images = Vec::with_capacity(transient_image_descriptions.len());

        for image_description in transient_image_descriptions {
            let image_extent = get_transient_image_size(
                image_description.size.clone(),
                persistent,
                swapchain_images,
                transient_image_descriptions,
            );

            let image = Image::new_2d(
                &self.device,
                ImageDescription2D::from_transient(
                    [image_extent.width, image_extent.height],
                    image_description,
                ),
            );

            let vk_image = VkImage {
                handle: image.handle,
                view: image.view,
                size: image.extend,
                format: image.format,
            };

            self.transient_images.push(image);
            transient_images.push(vk_image);
        }

        transient_images
    }

    pub(crate) fn flush(&mut self) {
        for buffer in self.transient_buffers.drain(..) {
            buffer.delete(&self.device);
        }

        for image in self.transient_images.drain(..) {
            image.delete(&self.device);
        }
    }
}

impl Drop for TransientResourceManager {
    fn drop(&mut self) {
        self.flush();
    }
}

fn get_transient_image_size(
    size: TransientImageSize,
    persistent: &PersistentResourceManager,
    swapchain_images: &[(vk::SwapchainKHR, SwapchainImage)],
    transient_image_descriptions: &[TransientImageDesc],
) -> vk::Extent2D {
    match size {
        TransientImageSize::Exact(extent) => extent,
        TransientImageSize::Relative(scale, target) => {
            let mut extent = match target {
                ImageHandle::Persistent(image_key) => {
                    persistent.get_image(image_key).as_ref().unwrap().extend
                }
                ImageHandle::Transient(index) => get_transient_image_size(
                    transient_image_descriptions[index].size.clone(),
                    persistent,
                    swapchain_images,
                    transient_image_descriptions,
                ),
                ImageHandle::Swapchain(index) => swapchain_images[index].1.extent,
            };
            extent.width = ((extent.width as f32) * scale[0]) as u32;
            extent.height = ((extent.height as f32) * scale[1]) as u32;

            extent
        }
    }
}

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
