use crate::buffer::{Buffer, BufferDescription};
use crate::descriptor_set::{DescriptorCount, DescriptorSet};
use crate::device::AshDevice;
use crate::image::{Image, ImageDescription2D};
use crate::render_graph::{TransientImageDesc, TransientImageSize, VkImage};
use crate::swapchain::SwapchainImage;
use crate::{BufferKey, ImageHandle, ImageKey};
use ash::vk;
use log::warn;
use slotmap::SlotMap;
use std::sync::Arc;

pub struct AshBufferResource {
    buffer: Buffer,
}

pub struct AshImageHandle {
    image: Image,
}

pub struct PersistentResourceManager {
    device: Arc<AshDevice>,

    buffers: SlotMap<BufferKey, AshBufferResource>,
    freed_buffers: Vec<BufferKey>,

    images: SlotMap<ImageKey, AshImageHandle>,
    freed_images: Vec<ImageKey>,

    pub(crate) descriptor_set: DescriptorSet,
}

impl PersistentResourceManager {
    pub fn new(device: Arc<AshDevice>) -> Self {
        let descriptor_set = DescriptorSet::new(
            device.clone(),
            DescriptorCount {
                storage_buffers: 1024,
                storage_images: 1024,
                sampled_images: 1024,
                ..Default::default()
            },
        )
        .unwrap();

        Self {
            device,
            buffers: SlotMap::with_key(),
            freed_buffers: Vec::new(),
            images: SlotMap::with_key(),
            freed_images: Vec::new(),
            descriptor_set,
        }
    }

    pub fn flush_frame(&mut self) {
        for key in self.freed_buffers.drain(..) {
            if self.buffers.remove(key).is_some() {
                warn!("BufferKey({:?}) was invalid on deletion", key);
            }
        }
        for key in self.freed_images.drain(..) {
            if self.images.remove(key).is_some() {
                warn!("ImageKey({:?}) was invalid on deletion", key);
            }
        }
    }

    pub fn add_buffer(&mut self, mut buffer: Buffer) -> BufferKey {
        if buffer.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
            buffer.storage_binding = Some(self.descriptor_set.bind_storage_buffer(&buffer));
        }

        self.buffers.insert(AshBufferResource { buffer })
    }

    pub fn get_buffer(&self, key: BufferKey) -> Option<&Buffer> {
        self.buffers.get(key).map(|resource| &resource.buffer)
    }

    pub fn remove_buffer(&mut self, key: BufferKey) {
        self.freed_buffers.push(key);
    }

    pub fn add_image(&mut self, mut image: Image) -> ImageKey {
        if image.usage.contains(vk::ImageUsageFlags::STORAGE) {
            image.storage_binding = Some(self.descriptor_set.bind_storage_image(&image));
        }

        self.images.insert(AshImageHandle { image })
    }

    pub fn get_image(&self, key: ImageKey) -> Option<&Image> {
        self.images.get(key).map(|resource| &resource.image)
    }

    pub fn remove_image(&mut self, key: ImageKey) {
        self.freed_images.push(key);
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
                Buffer::new(self.device.clone(), "Transient Buffer", buffer_description)
                    .expect("TODO: replace this"),
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
                self.device.clone(),
                "Transient Image",
                &ImageDescription2D::from_transient(
                    [image_extent.width, image_extent.height],
                    image_description,
                ),
            )
            .expect("TODO: replace this");

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
        self.transient_buffers.clear();
        self.transient_images.clear();
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
