use crate::buffer::{AshBuffer, Buffer};
use crate::descriptor_set::{DescriptorCount, DescriptorSet};
use crate::device::AshDevice;
use crate::image::{AshImage, Image, TransientImageSize};
use crate::render_graph::{
    BufferResourceDescription, BufferResourceUsage, ImageResourceDescription, ImageResourceUsage,
};
use crate::sampler::Sampler;
use crate::swapchain::AcquiredSwapchainImage;
use crate::{BufferKey, ImageHandle, ImageKey, SamplerKey, VulkanError};
use ash::vk;
use log::warn;
use slotmap::SlotMap;
use std::sync::Arc;

pub struct BufferResource {
    buffer: Buffer,
}

pub struct BufferGraphResource {
    pub buffer: AshBuffer,
    last_usage: BufferResourceUsage,
}

pub struct ImageResource {
    image: Image,
}

pub struct ImageGraphResource {
    pub image: AshImage,
    pub last_usage: ImageResourceUsage,
    pub layout: vk::ImageLayout,
}

pub struct ResourceManager {
    #[allow(unused)]
    device: Arc<AshDevice>,

    buffers: SlotMap<BufferKey, BufferResource>,
    freed_buffers: Vec<BufferKey>,

    images: SlotMap<ImageKey, ImageResource>,
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

        self.buffers.insert(BufferResource { buffer })
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

        self.images.insert(ImageResource { image })
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

    //Graph Functions

    //TODO: take in vector to reuse memory?
    /// Get the buffer resources and update the last usages
    pub fn get_buffer_resources(
        &mut self,
        buffers: &[BufferResourceDescription],
    ) -> Result<Vec<BufferGraphResource>, VulkanError> {
        let mut buffer_resources = Vec::with_capacity(buffers.len());
        for buffer_description in buffers {
            buffer_resources.push(match buffer_description {
                BufferResourceDescription::Persistent(key) => {
                    let buffer = &self.buffers[*key];
                    //TODO: get usages with multiple frames in flight
                    //TODO: write last usages + queue
                    BufferGraphResource {
                        buffer: buffer.buffer.get_copy(),
                        last_usage: BufferResourceUsage::None,
                    }
                }
                BufferResourceDescription::Transient(buffer_description) => {
                    let mut buffer =
                        Buffer::new(self.device.clone(), "Transient Buffer", buffer_description)?;
                    if buffer.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
                        buffer.storage_binding =
                            Some(self.descriptor_set.bind_storage_buffer(&buffer));
                    }
                    let resource = BufferGraphResource {
                        buffer: buffer.get_copy(),
                        last_usage: BufferResourceUsage::None, //Never used before
                    };
                    self.transient_buffers.push(buffer);
                    resource
                }
            });
        }

        Ok(buffer_resources)
    }

    //TODO: take in vector to reuse memory?
    /// Get the image resources and update the last usages
    pub fn get_image_resources(
        &mut self,
        swapchain_images: &[AcquiredSwapchainImage],
        images: &[ImageResourceDescription],
    ) -> Result<Vec<ImageGraphResource>, VulkanError> {
        let mut image_resources = Vec::with_capacity(images.len());
        for image_description in images {
            image_resources.push(match image_description {
                ImageResourceDescription::Persistent(key) => {
                    let image = &self.images[*key];
                    //TODO: get usages with multiple frames in flight
                    //TODO: write last usages + queue + layout
                    ImageGraphResource {
                        image: image.image.get_copy(),
                        last_usage: ImageResourceUsage::None,
                        layout: vk::ImageLayout::UNDEFINED,
                    }
                }
                ImageResourceDescription::Transient(transient_image_description) => {
                    let image_size = get_transient_image_size(
                        transient_image_description.size.clone(),
                        self,
                        swapchain_images,
                    );
                    let image_description = transient_image_description
                        .to_image_description([image_size.width, image_size.height]);
                    let mut image =
                        Image::new_2d(self.device.clone(), "Transient Image", &image_description)?;

                    if image.usage.contains(vk::ImageUsageFlags::STORAGE) {
                        image.storage_binding =
                            Some(self.descriptor_set.bind_storage_image(&image));
                    }

                    if image.usage.contains(vk::ImageUsageFlags::SAMPLED) {
                        image.sampled_binding =
                            Some(self.descriptor_set.bind_sampled_image(&image));
                    }

                    let resource = ImageGraphResource {
                        image: image.get_copy(),
                        last_usage: ImageResourceUsage::None, //Never used before
                        layout: vk::ImageLayout::UNDEFINED,
                    };
                    self.transient_images.push(image);
                    resource
                }
                ImageResourceDescription::Swapchain(index) => {
                    //Swapchain always starts out unused
                    ImageGraphResource {
                        image: swapchain_images[*index].image,
                        last_usage: ImageResourceUsage::None,
                        layout: vk::ImageLayout::UNDEFINED,
                    }
                }
            });
        }

        Ok(image_resources)
    }
}

fn get_transient_image_size(
    size: TransientImageSize,
    persistent: &ResourceManager,
    swapchain_images: &[AcquiredSwapchainImage],
) -> vk::Extent2D {
    match size {
        TransientImageSize::Exact(extent) => extent,
        TransientImageSize::Relative(scale, target) => {
            let mut extent = match target {
                ImageHandle::Persistent(image_key) => {
                    persistent.get_image(image_key).as_ref().unwrap().size
                }
                ImageHandle::Transient(index) => {
                    todo!("Need to switch index to a ImageIndex");
                    //     get_transient_image_size(
                    //     transient_image_descriptions[index].size.clone(),
                    //     persistent,
                    //     swapchain_images,
                    // )
                }
                ImageHandle::Swapchain(index) => swapchain_images[index].image.size,
            };
            extent.width = ((extent.width as f32) * scale[0]) as u32;
            extent.height = ((extent.height as f32) * scale[1]) as u32;

            extent
        }
    }
}
