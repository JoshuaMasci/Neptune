use crate::buffer::{Buffer, BufferDescription};
use crate::descriptor_set::{DescriptorCount, DescriptorSet};
use crate::device::AshDevice;
use crate::image::{Image, ImageDescription2D, TransientImageDesc, TransientImageSize};
use crate::sampler::Sampler;
use crate::swapchain::AcquiredSwapchainImage;
use crate::{BufferKey, ImageHandle, ImageKey, SamplerKey, VulkanError};
use ash::vk;
use log::{info, warn};
use slotmap::SlotMap;
use std::sync::Arc;

pub struct AshBufferResource {
    buffer: Buffer,
}

pub struct AshImageHandle {
    image: Image,
}

pub struct ResourceManager {
    #[allow(unused)]
    device: Arc<AshDevice>,

    buffers: SlotMap<BufferKey, AshBufferResource>,
    freed_buffers: Vec<BufferKey>,

    images: SlotMap<ImageKey, AshImageHandle>,
    freed_images: Vec<ImageKey>,

    samplers: SlotMap<SamplerKey, Arc<Sampler>>,

    pub(crate) descriptor_set: DescriptorSet,

    //TODO: rework this use multiple frames in flight
    freed_buffers2: Vec<BufferKey>,
    freed_images2: Vec<ImageKey>,
    pub(crate) transient_buffers: Vec<Buffer>,
    pub(crate) transient_images: Vec<Image>,
}

impl ResourceManager {
    pub fn new(device: Arc<AshDevice>) -> Self {
        let descriptor_set = DescriptorSet::new(
            device.clone(),
            DescriptorCount {
                storage_buffers: 1024,
                storage_images: 1024,
                sampled_images: 1024,
                samplers: 128,
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
            samplers: SlotMap::with_key(),
            descriptor_set,
            freed_buffers2: Vec::new(),
            freed_images2: Vec::new(),
            transient_buffers: Vec::new(),
            transient_images: Vec::new(),
        }
    }

    pub fn flush_frame(&mut self) {
        //TODO: fix this when multiple frames in flight implemented
        for key in self.freed_buffers2.drain(..) {
            if self.buffers.remove(key).is_none() {
                warn!("BufferKey({:?}) was invalid on deletion", key);
            }
        }
        for key in self.freed_images2.drain(..) {
            if self.images.remove(key).is_none() {
                warn!("ImageKey({:?}) was invalid on deletion", key);
            }
        }
        self.freed_buffers2 = std::mem::take(&mut self.freed_buffers);
        self.freed_images2 = std::mem::take(&mut self.freed_images);

        self.transient_buffers.clear();
        self.transient_images.clear();
    }

    //Buffers
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

    //Images
    pub fn add_image(&mut self, mut image: Image) -> ImageKey {
        if image.usage.contains(vk::ImageUsageFlags::STORAGE) {
            image.storage_binding = Some(self.descriptor_set.bind_storage_image(&image));
        }

        if image.usage.contains(vk::ImageUsageFlags::SAMPLED) {
            image.sampled_binding = Some(self.descriptor_set.bind_sampled_image(&image));
        }

        self.images.insert(AshImageHandle { image })
    }
    pub fn get_image(&self, key: ImageKey) -> Option<&Image> {
        self.images.get(key).map(|resource| &resource.image)
    }
    pub fn remove_image(&mut self, key: ImageKey) {
        self.freed_images.push(key);
    }

    //Samplers
    pub fn add_sampler(&mut self, mut sampler: Sampler) -> SamplerKey {
        sampler.binding = Some(self.descriptor_set.bind_sampler(&sampler));
        self.samplers.insert(Arc::new(sampler))
    }
    pub fn get_sampler(&self, key: SamplerKey) -> Option<Arc<Sampler>> {
        self.samplers.get(key).cloned()
    }
    pub fn remove_sampler(&mut self, key: SamplerKey) {
        if self.samplers.remove(key).is_none() {
            warn!("Tried to remove invalid SamplerKey({:?})", key);
        }
    }

    //Transient resources
    pub fn resolve_transient_buffers(
        &mut self,
        transient_image_descriptions: &[BufferDescription],
    ) -> Result<(), VulkanError> {
        for buffer_description in transient_image_descriptions {
            let mut buffer =
                Buffer::new(self.device.clone(), "Transient Buffer", buffer_description)?;

            if buffer.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
                buffer.storage_binding = Some(self.descriptor_set.bind_storage_buffer(&buffer));
            }

            self.transient_buffers.push(buffer);
        }

        Ok(())
    }
    pub fn resolve_transient_image(
        &mut self,
        swapchain_images: &[AcquiredSwapchainImage],
        transient_image_descriptions: &[TransientImageDesc],
    ) -> Result<(), VulkanError> {
        for image_description in transient_image_descriptions {
            let image_extent = get_transient_image_size(
                image_description.size.clone(),
                self,
                swapchain_images,
                transient_image_descriptions,
            );

            let mut image = Image::new_2d(
                self.device.clone(),
                "Transient Image",
                &ImageDescription2D::from_transient(
                    [image_extent.width, image_extent.height],
                    image_description,
                ),
            )?;

            if image.usage.contains(vk::ImageUsageFlags::STORAGE) {
                image.storage_binding = Some(self.descriptor_set.bind_storage_image(&image));
            }

            if image.usage.contains(vk::ImageUsageFlags::SAMPLED) {
                image.sampled_binding = Some(self.descriptor_set.bind_sampled_image(&image));
            }

            self.transient_images.push(image);
        }

        Ok(())
    }
}

fn get_transient_image_size(
    size: TransientImageSize,
    persistent: &ResourceManager,
    swapchain_images: &[AcquiredSwapchainImage],
    transient_image_descriptions: &[TransientImageDesc],
) -> vk::Extent2D {
    match size {
        TransientImageSize::Exact(extent) => extent,
        TransientImageSize::Relative(scale, target) => {
            let mut extent = match target {
                ImageHandle::Persistent(image_key) => {
                    persistent.get_image(image_key).as_ref().unwrap().size
                }
                ImageHandle::Transient(index) => get_transient_image_size(
                    transient_image_descriptions[index].size.clone(),
                    persistent,
                    swapchain_images,
                    transient_image_descriptions,
                ),
                ImageHandle::Swapchain(index) => swapchain_images[index].image.size,
            };
            extent.width = ((extent.width as f32) * scale[0]) as u32;
            extent.height = ((extent.height as f32) * scale[1]) as u32;

            extent
        }
    }
}
