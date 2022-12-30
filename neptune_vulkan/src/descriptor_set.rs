use crate::{AshBuffer, AshDevice, AshImage, AshSampler};
use ash::vk;
use neptune_core::IndexPool;
use std::sync::Arc;

const EMPTY_BUFFER_INFO: vk::DescriptorBufferInfo = vk::DescriptorBufferInfo {
    buffer: vk::Buffer::null(),
    offset: 0,
    range: vk::WHOLE_SIZE,
};

const EMPTY_IMAGE_INFO: vk::DescriptorImageInfo = vk::DescriptorImageInfo {
    sampler: vk::Sampler::null(),
    image_view: vk::ImageView::null(),
    image_layout: vk::ImageLayout::UNDEFINED,
};

#[derive(Debug, Default)]
pub(crate) struct BindingCount {
    pub(crate) uniform_buffers: u32,
    pub(crate) storage_buffers: u32,
    pub(crate) sampled_textures: u32,
    pub(crate) storage_textures: u32,

    #[allow(dead_code)]
    pub(crate) acceleration_structures: u32,
}

struct DescriptorArray<T> {
    index_pool: IndexPool<u32>,
    updates: Vec<(u32, T)>,
}

impl<T> DescriptorArray<T> {
    pub fn new(range: std::ops::Range<u32>) -> Self {
        Self {
            index_pool: IndexPool::new_range(range),
            updates: Vec::new(),
        }
    }
}

pub(crate) struct DescriptorSet {
    device: Arc<AshDevice>,
    layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    set: vk::DescriptorSet,

    uniform_buffers: DescriptorArray<vk::DescriptorBufferInfo>,
    storage_buffers: DescriptorArray<vk::DescriptorBufferInfo>,
    sampled_images: DescriptorArray<vk::DescriptorImageInfo>,
    storage_images: DescriptorArray<vk::DescriptorImageInfo>,
}

impl DescriptorSet {
    const UNIFORM_BUFFER_BINDING: u32 = 0;
    const STORAGE_BUFFER_BINDING: u32 = 1;
    const SAMPLED_IMAGE_BINDING: u32 = 2;
    const STORAGE_IMAGE_BINDING: u32 = 3;

    #[allow(dead_code)]
    const ACCELERATION_STRUCTURE_BINDING: u32 = 4;

    pub(crate) fn new(device: Arc<AshDevice>, counts: BindingCount) -> crate::Result<Self> {
        let binding_flags = [vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            | vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING;
            4];
        let mut binding_flag_create_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
            .binding_flags(&binding_flags)
            .build();

        let layout = match unsafe {
            device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder()
                    .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
                    .bindings(&[
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(Self::UNIFORM_BUFFER_BINDING)
                            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                            .descriptor_count(counts.uniform_buffers)
                            .stage_flags(vk::ShaderStageFlags::ALL)
                            .build(),
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(Self::STORAGE_BUFFER_BINDING)
                            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                            .descriptor_count(counts.storage_buffers)
                            .stage_flags(vk::ShaderStageFlags::ALL)
                            .build(),
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(Self::SAMPLED_IMAGE_BINDING)
                            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                            .descriptor_count(counts.sampled_textures)
                            .stage_flags(vk::ShaderStageFlags::ALL)
                            .build(),
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(Self::STORAGE_IMAGE_BINDING)
                            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                            .descriptor_count(counts.storage_textures)
                            .stage_flags(vk::ShaderStageFlags::ALL)
                            .build(),
                    ])
                    .push_next(&mut binding_flag_create_info)
                    .build(),
                None,
            )
        } {
            Ok(layout) => layout,
            Err(e) => return Err(crate::Error::VkError(e)),
        };

        let pool = match unsafe {
            device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::builder()
                    .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
                    .max_sets(1)
                    .pool_sizes(&[
                        vk::DescriptorPoolSize::builder()
                            .ty(vk::DescriptorType::UNIFORM_BUFFER)
                            .descriptor_count(counts.uniform_buffers)
                            .build(),
                        vk::DescriptorPoolSize::builder()
                            .ty(vk::DescriptorType::STORAGE_BUFFER)
                            .descriptor_count(counts.storage_buffers)
                            .build(),
                        vk::DescriptorPoolSize::builder()
                            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                            .descriptor_count(counts.sampled_textures)
                            .build(),
                        vk::DescriptorPoolSize::builder()
                            .ty(vk::DescriptorType::STORAGE_IMAGE)
                            .descriptor_count(counts.storage_textures)
                            .build(),
                    ])
                    .build(),
                None,
            )
        } {
            Ok(pool) => pool,
            Err(e) => {
                unsafe {
                    device.destroy_descriptor_set_layout(layout, None);
                }
                return Err(crate::Error::VkError(e));
            }
        };

        let set = match unsafe {
            device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(pool)
                    .set_layouts(&[layout]),
            )
        } {
            Ok(set) => set[0],
            Err(e) => {
                unsafe {
                    device.destroy_descriptor_set_layout(layout, None);
                    device.destroy_descriptor_pool(pool, None);
                }
                return Err(crate::Error::VkError(e));
            }
        };

        Ok(Self {
            device,
            layout,
            pool,
            set,
            uniform_buffers: DescriptorArray::new(0..counts.uniform_buffers),
            storage_buffers: DescriptorArray::new(0..counts.storage_buffers),
            sampled_images: DescriptorArray::new(0..counts.sampled_textures),
            storage_images: DescriptorArray::new(0..counts.storage_textures),
        })
    }

    pub(crate) fn layout(&self) -> vk::DescriptorSetLayout {
        self.layout
    }

    pub(crate) fn set(&self) -> vk::DescriptorSet {
        self.set
    }

    pub(crate) fn update(&mut self) {
        let mut writes: Vec<vk::WriteDescriptorSet> = Vec::with_capacity(
            self.uniform_buffers.updates.len()
                + self.storage_buffers.updates.len()
                + self.sampled_images.updates.len()
                + self.storage_images.updates.len(),
        );

        for (index, info) in self.uniform_buffers.updates.iter() {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::UNIFORM_BUFFER_BINDING,
                dst_array_element: *index,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_buffer_info: info,
                ..Default::default()
            });
        }

        for (index, info) in self.storage_buffers.updates.iter() {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::STORAGE_BUFFER_BINDING,
                dst_array_element: *index,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_buffer_info: info,
                ..Default::default()
            });
        }

        for (index, info) in self.sampled_images.updates.iter() {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::SAMPLED_IMAGE_BINDING,
                dst_array_element: *index,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: info,
                ..Default::default()
            });
        }

        for (index, info) in self.storage_images.updates.iter() {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::STORAGE_IMAGE_BINDING,
                dst_array_element: *index,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                p_image_info: info,
                ..Default::default()
            });
        }

        //TODO: profile this part
        unsafe {
            self.device.update_descriptor_sets(&writes, &[]);
        }

        self.uniform_buffers.updates.clear();
        self.storage_buffers.updates.clear();
        self.sampled_images.updates.clear();
        self.storage_images.updates.clear();
    }

    pub(crate) fn bind_uniform_buffer(&mut self, buffer: &AshBuffer) -> crate::Result<u32> {
        match self.uniform_buffers.index_pool.get() {
            None => Err(crate::Error::StringError(
                "Out of Uniform Buffer Descriptor Slots!".to_string(),
            )),
            Some(index) => {
                self.uniform_buffers.updates.push((
                    index,
                    vk::DescriptorBufferInfo {
                        buffer: buffer.handle,
                        offset: 0,
                        range: vk::WHOLE_SIZE,
                    },
                ));
                Ok(index)
            }
        }
    }

    pub(crate) fn unbind_uniform_buffer(&mut self, index: u32) {
        self.uniform_buffers
            .updates
            .push((index, self::EMPTY_BUFFER_INFO));
        self.uniform_buffers.index_pool.free(index);
    }

    pub(crate) fn bind_storage_buffer(&mut self, buffer: &AshBuffer) -> crate::Result<u32> {
        match self.storage_buffers.index_pool.get() {
            None => Err(crate::Error::StringError(
                "Out of Storage Buffer Descriptor Slots!".to_string(),
            )),
            Some(index) => {
                self.storage_buffers.updates.push((
                    index,
                    vk::DescriptorBufferInfo {
                        buffer: buffer.handle,
                        offset: 0,
                        range: vk::WHOLE_SIZE,
                    },
                ));
                Ok(index)
            }
        }
    }

    pub(crate) fn unbind_storage_buffer(&mut self, index: u32) {
        self.storage_buffers
            .updates
            .push((index, self::EMPTY_BUFFER_INFO));
        self.storage_buffers.index_pool.free(index);
    }

    pub(crate) fn bind_sampled_image(
        &mut self,
        image: &AshImage,
        sampler: &Arc<AshSampler>,
    ) -> crate::Result<u32> {
        match self.sampled_images.index_pool.get() {
            None => Err(crate::Error::StringError(
                "Out of Sampled Image Descriptor Slots!".to_string(),
            )),
            Some(index) => {
                self.sampled_images.updates.push((
                    index,
                    vk::DescriptorImageInfo {
                        sampler: sampler.handle,
                        image_view: image.view,
                        image_layout: vk::ImageLayout::GENERAL,
                    },
                ));
                Ok(index)
            }
        }
    }

    pub(crate) fn unbind_sampled_image(&mut self, index: u32) {
        self.sampled_images
            .updates
            .push((index, self::EMPTY_IMAGE_INFO));
        self.sampled_images.index_pool.free(index);
    }

    pub(crate) fn bind_storage_image(&mut self, image: &AshImage) -> crate::Result<u32> {
        match self.storage_images.index_pool.get() {
            None => Err(crate::Error::StringError(
                "Out of Storage Image Descriptor Slots!".to_string(),
            )),
            Some(index) => {
                self.storage_images.updates.push((
                    index,
                    vk::DescriptorImageInfo {
                        sampler: vk::Sampler::null(),
                        image_view: image.view,
                        image_layout: vk::ImageLayout::GENERAL,
                    },
                ));
                Ok(index)
            }
        }
    }

    pub(crate) fn unbind_storage_image(&mut self, index: u32) {
        self.storage_images
            .updates
            .push((index, self::EMPTY_IMAGE_INFO));
        self.storage_images.index_pool.free(index);
    }
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_pool(self.pool, None);
            self.device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}
