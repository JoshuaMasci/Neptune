use crate::{AshBuffer, AshDevice, AshImage, AshSampler, SamplerCreateInfo};
use ash::prelude::VkResult;
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
    pub(crate) samplers: u32,
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

    //Sampler Doesn't support nullDescriptor, this will serve as the empty value
    pub(crate) null_sampler: AshSampler,

    uniform_buffers: DescriptorArray<vk::DescriptorBufferInfo>,
    storage_buffers: DescriptorArray<vk::DescriptorBufferInfo>,
    sampled_images: DescriptorArray<vk::DescriptorImageInfo>,
    storage_images: DescriptorArray<vk::DescriptorImageInfo>,
    samplers: DescriptorArray<vk::DescriptorImageInfo>,
}

impl DescriptorSet {
    const UNIFORM_BUFFER_BINDING: u32 = 0;
    const STORAGE_BUFFER_BINDING: u32 = 1;
    const SAMPLED_IMAGE_BINDING: u32 = 2;
    const STORAGE_IMAGE_BINDING: u32 = 3;
    const SAMPLER_BINDING: u32 = 4;
    const ACCELERATION_STRUCTURE_BINDING: u32 = 5;

    pub(crate) fn new(device: Arc<AshDevice>, counts: BindingCount) -> crate::Result<Self> {
        let binding_flags = [vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            | vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING;
            5];
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
                            .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                            .descriptor_count(counts.sampled_textures)
                            .stage_flags(vk::ShaderStageFlags::ALL)
                            .build(),
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(Self::STORAGE_IMAGE_BINDING)
                            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                            .descriptor_count(counts.storage_textures)
                            .stage_flags(vk::ShaderStageFlags::ALL)
                            .build(),
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(Self::SAMPLER_BINDING)
                            .descriptor_type(vk::DescriptorType::SAMPLER)
                            .descriptor_count(counts.samplers)
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
                            .ty(vk::DescriptorType::SAMPLED_IMAGE)
                            .descriptor_count(counts.sampled_textures)
                            .build(),
                        vk::DescriptorPoolSize::builder()
                            .ty(vk::DescriptorType::STORAGE_IMAGE)
                            .descriptor_count(counts.storage_textures)
                            .build(),
                        vk::DescriptorPoolSize::builder()
                            .ty(vk::DescriptorType::SAMPLER)
                            .descriptor_count(counts.samplers)
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

        let null_sampler = AshSampler::new(&device, &SamplerCreateInfo::default())?;

        write_empty_image_info(
            &device,
            set,
            Self::SAMPLER_BINDING,
            vk::DescriptorType::SAMPLER,
            0..counts.samplers,
            &vk::DescriptorImageInfo {
                sampler: null_sampler.handle,
                image_view: vk::ImageView::null(),
                image_layout: vk::ImageLayout::UNDEFINED,
            },
        );

        Ok(Self {
            device,
            layout,
            pool,
            set,
            null_sampler,
            uniform_buffers: DescriptorArray::new(0..counts.uniform_buffers),
            storage_buffers: DescriptorArray::new(0..counts.storage_buffers),
            sampled_images: DescriptorArray::new(0..counts.sampled_textures),
            storage_images: DescriptorArray::new(0..counts.storage_textures),
            samplers: DescriptorArray::new(0..counts.samplers),
        })
    }

    pub(crate) fn layout(&self) -> vk::DescriptorSetLayout {
        self.layout
    }

    pub(crate) fn set(&self) -> vk::DescriptorSet {
        self.set
    }

    pub(crate) fn update(&mut self) {
        let mut writes: Vec<vk::WriteDescriptorSet> =
            Vec::with_capacity(self.uniform_buffers.updates.len() + self.samplers.updates.len());

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

        for (index, info) in self.samplers.updates.iter() {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::SAMPLER_BINDING,
                dst_array_element: *index,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::SAMPLER,
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
        self.samplers.updates.clear();
    }

    pub(crate) fn bind_uniform_buffer(&mut self, buffer: AshBuffer) -> crate::Result<u32> {
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

    pub(crate) fn bind_sampler(&mut self, sampler: vk::Sampler) -> crate::Result<u32> {
        match self.samplers.index_pool.get() {
            None => Err(crate::Error::StringError(
                "Out of Uniform Buffer Descriptor Slots!".to_string(),
            )),
            Some(index) => {
                self.samplers.updates.push((
                    index,
                    vk::DescriptorImageInfo {
                        sampler,
                        image_view: vk::ImageView::null(),
                        image_layout: vk::ImageLayout::UNDEFINED,
                    },
                ));
                Ok(index)
            }
        }
    }

    pub(crate) fn unbind_sampler(&mut self, index: u32) {
        self.samplers.updates.push((
            index,
            vk::DescriptorImageInfo {
                sampler: self.null_sampler.handle,
                image_view: vk::ImageView::null(),
                image_layout: vk::ImageLayout::UNDEFINED,
            },
        ));
        self.samplers.index_pool.free(index);
    }
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_pool(self.pool, None);
            self.device.destroy_descriptor_set_layout(self.layout, None);
            self.null_sampler.destroy(&self.device);
        }
    }
}

fn write_empty_image_info(
    device: &Arc<AshDevice>,
    descriptor_set: vk::DescriptorSet,
    descriptor_binding: u32,
    descriptor_type: vk::DescriptorType,
    range: std::ops::Range<u32>,
    image_info: &vk::DescriptorImageInfo,
) {
    let writes: Vec<vk::WriteDescriptorSet> = range
        .map(|i| vk::WriteDescriptorSet {
            dst_set: descriptor_set,
            dst_binding: descriptor_binding,
            dst_array_element: i,
            descriptor_count: 1,
            descriptor_type,
            p_image_info: image_info,
            ..Default::default()
        })
        .collect();

    //TODO: profile this part
    unsafe {
        device.update_descriptor_sets(&writes, &[]);
    }
}
