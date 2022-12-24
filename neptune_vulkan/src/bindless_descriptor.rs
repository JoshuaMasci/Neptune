use crate::AshDevice;
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
    uniform_buffers: u32,
    storage_buffers: u32,
    sampled_textures: u32,
    storage_textures: u32,
    samplers: u32,
    acceleration_structures: u32,
}

struct DescriptorArray<T> {
    index_pool: IndexPool<u32>,
    updates: Vec<(u32, T)>,
}

impl<T> DescriptorArray<T> {
    pub fn new(range: std::ops::Range<u32>) -> Self {
        Self {
            index_pool: IndexPool::new(range),
            updates: Vec::new(),
        }
    }
}

pub(crate) struct BindlessDescriptorSet {
    device: Arc<AshDevice>,
    layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    set: vk::DescriptorSet,

    uniform_buffers: DescriptorArray<vk::DescriptorBufferInfo>,
    storage_buffers: DescriptorArray<vk::DescriptorBufferInfo>,
    sampled_images: DescriptorArray<vk::DescriptorImageInfo>,
    storage_images: DescriptorArray<vk::DescriptorImageInfo>,
    samplers: DescriptorArray<vk::DescriptorImageInfo>,
}

impl BindlessDescriptorSet {
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

        Ok(Self {
            device,
            layout,
            pool,
            set,
            uniform_buffers: DescriptorArray::new(0..counts.uniform_buffers),
            storage_buffers: DescriptorArray::new(0..counts.storage_buffers),
            sampled_images: DescriptorArray::new(0..counts.sampled_textures),
            storage_images: DescriptorArray::new(0..counts.storage_textures),
            samplers: DescriptorArray::new(0..counts.samplers),
        })
    }
}
