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
    BasicRenderGraphExecutor, BufferAccess, BufferResource, BuildCommandFn, ColorAttachment,
    DepthStencilAttachment, Framebuffer, ImageAccess, ImageResource, RenderGraph,
    RenderGraphResources, RenderPass, TransientImageDesc, TransientImageSize,
};
pub use crate::swapchain::{AshSwapchain, AshSwapchainSettings, SwapchainManager};

pub use ash::vk;
pub use gpu_allocator;

use crate::render_graph::VkImage;
use crate::swapchain::AshSwapchainImage;
use log::{info, warn};
use slotmap::{new_key_type, SlotMap};
use std::collections::HashMap;
use std::sync::Arc;

new_key_type! {
   pub struct BufferKey;
   pub struct ImageKey;
}

pub struct AshBuffer {
    pub handle: vk::Buffer,
    pub allocation: gpu_allocator::vulkan::Allocation,
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub location: gpu_allocator::MemoryLocation,
}

impl AshBuffer {
    pub fn new(
        device: &AshDevice,
        create_info: &vk::BufferCreateInfo,
        location: gpu_allocator::MemoryLocation,
    ) -> Self {
        let handle =
            unsafe { device.core.create_buffer(create_info, None) }.expect("TODO: return error");

        let requirements = unsafe { device.core.get_buffer_memory_requirements(handle) };

        let allocation = device
            .allocator
            .lock()
            .unwrap()
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "Buffer Allocation",
                requirements,
                location,
                linear: true,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .expect("TODO: return error");

        unsafe {
            device
                .core
                .bind_buffer_memory(handle, allocation.memory(), allocation.offset())
                .expect("TODO: return error");
        }

        Self {
            handle,
            allocation,
            size: create_info.size,
            usage: create_info.usage,
            location,
        }
    }

    pub fn delete(self, device: &AshDevice) {
        unsafe {
            device.core.destroy_buffer(self.handle, None);
        };

        let _ = device.allocator.lock().unwrap().free(self.allocation);
    }
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

pub fn vk_format_get_aspect_flags(format: vk::Format) -> vk::ImageAspectFlags {
    match format {
        vk::Format::D16_UNORM | vk::Format::D32_SFLOAT | vk::Format::X8_D24_UNORM_PACK32 => {
            vk::ImageAspectFlags::DEPTH
        }
        vk::Format::S8_UINT => vk::ImageAspectFlags::STENCIL,
        vk::Format::D32_SFLOAT_S8_UINT | vk::Format::D24_UNORM_S8_UINT => {
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
        }
        _ => vk::ImageAspectFlags::COLOR,
    }
}

pub struct AshImage {
    pub handle: vk::Image,
    pub view: vk::ImageView,
    pub allocation: gpu_allocator::vulkan::Allocation,
    pub extend: vk::Extent2D,
    pub format: vk::Format,
    pub usage: vk::ImageUsageFlags,
    pub location: gpu_allocator::MemoryLocation,
}

impl AshImage {
    pub fn new(
        device: &AshDevice,
        create_info: &vk::ImageCreateInfo,
        view_create_info: &vk::ImageViewCreateInfo,
        location: gpu_allocator::MemoryLocation,
    ) -> Self {
        let handle =
            unsafe { device.core.create_image(create_info, None) }.expect("TODO: return error");

        let requirements = unsafe { device.core.get_image_memory_requirements(handle) };

        let allocation = device
            .allocator
            .lock()
            .unwrap()
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "Image Allocation",
                requirements,
                location,
                linear: true,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .expect("TODO: return error");

        unsafe {
            device
                .core
                .bind_image_memory(handle, allocation.memory(), allocation.offset())
                .expect("TODO: return error");
        }

        let mut view_create_info = *view_create_info;
        view_create_info.image = handle;

        let view = unsafe { device.core.create_image_view(&view_create_info, None) }
            .expect("TODO: return error");

        Self {
            handle,
            view,
            allocation,
            extend: vk::Extent2D {
                width: create_info.extent.width,
                height: create_info.extent.height,
            },
            format: create_info.format,
            usage: create_info.usage,
            location,
        }
    }

    pub fn new_desc(
        device: &AshDevice,
        desc: &TransientImageDesc,
        resolved_extent: vk::Extent2D,
    ) -> Self {
        Self::new(
            device,
            &vk::ImageCreateInfo::builder()
                .format(desc.format)
                .extent(vk::Extent3D {
                    width: resolved_extent.width,
                    height: resolved_extent.height,
                    depth: 1,
                })
                .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                .array_layers(1)
                .mip_levels(desc.mip_levels)
                .samples(vk::SampleCountFlags::TYPE_1)
                .image_type(vk::ImageType::TYPE_2D),
            &vk::ImageViewCreateInfo::builder()
                .format(desc.format)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk_format_get_aspect_flags(desc.format),
                    base_mip_level: 0,
                    level_count: desc.mip_levels,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .view_type(vk::ImageViewType::TYPE_2D),
            desc.memory_location,
        )
    }

    pub fn delete(self, device: &AshDevice) {
        unsafe {
            device.core.destroy_image_view(self.view, None);
            device.core.destroy_image(self.handle, None);
        };

        let _ = device.allocator.lock().unwrap().free(self.allocation);
    }
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
    device: Arc<AshDevice>,

    current_frame_index: usize,
    frames: Vec<ResourceFrame>,

    buffers: SlotMap<BufferKey, AshBufferResource>,
    images: SlotMap<ImageKey, AshImageResource>,
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

    transient_images: Vec<AshImage>,
}

impl TransientResourceManager {
    pub fn new(device: Arc<AshDevice>) -> Self {
        Self {
            device,
            transient_images: vec![],
        }
    }

    fn resolve_images(
        &mut self,
        persistent: &mut PersistentResourceManager,
        swapchain_images: &[(vk::SwapchainKHR, AshSwapchainImage)],
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

            let image = AshImage::new_desc(&self.device, image_description, image_extent);

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

    fn flush(&mut self) {
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
    swapchain_images: &[(vk::SwapchainKHR, AshSwapchainImage)],
    transient_image_descriptions: &[TransientImageDesc],
) -> vk::Extent2D {
    match size {
        TransientImageSize::Exact(extent) => extent,
        TransientImageSize::Relative(scale, target) => {
            let mut extent = match target {
                ImageResource::Persistent(image_key) => {
                    persistent.get_image(image_key).as_ref().unwrap().extend
                }
                ImageResource::Transient(index) => get_transient_image_size(
                    transient_image_descriptions[index].size.clone(),
                    persistent,
                    swapchain_images,
                    transient_image_descriptions,
                ),
                ImageResource::Swapchain(index) => swapchain_images[index].1.extent,
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
