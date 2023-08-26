use crate::buffer::Buffer;
use crate::device::AshDevice;
use crate::image::Image;
use crate::VulkanError;
use ash::vk;
use std::sync::{Arc, Mutex};

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

pub struct DescriptorCount {
    storage_buffers: u16,
    storage_images: u16,
    sampled_images: u16,
    acceleration_structures: u16,
}

pub struct DescriptorSet {
    device: Arc<AshDevice>,
    layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    set: vk::DescriptorSet,

    storage_buffer_pool: IndexPool,
    storage_image_pool: IndexPool,
    sampled_image_pool: IndexPool,
    acceleration_structure_pool: IndexPool,
}

impl DescriptorSet {
    const STORAGE_BUFFER_BINDING: u16 = 0;
    const SAMPLED_IMAGE_BINDING: u16 = 1;
    const STORAGE_IMAGE_BINDING: u16 = 2;
    const ACCELERATION_STRUCTURE_BINDING: u16 = 3;

    fn new(device: Arc<AshDevice>, count: DescriptorCount) -> Result<Self, VulkanError> {
        let mut bindings = Vec::new();
        let mut pool_sizes = Vec::new();

        if count.storage_buffers != 0 {
            bindings.push(vk::DescriptorSetLayoutBinding {
                binding: Self::STORAGE_BUFFER_BINDING as u32,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: count.storage_buffers as u32,
                stage_flags: vk::ShaderStageFlags::ALL,
                p_immutable_samplers: std::ptr::null(),
            });
            pool_sizes.push(vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: count.storage_buffers as u32,
            });
        }

        if count.storage_images != 0 {
            bindings.push(vk::DescriptorSetLayoutBinding {
                binding: Self::STORAGE_IMAGE_BINDING as u32,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: count.storage_images as u32,
                stage_flags: vk::ShaderStageFlags::ALL,
                p_immutable_samplers: std::ptr::null(),
            });
            pool_sizes.push(vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: count.storage_images as u32,
            });
        }

        if count.sampled_images != 0 {
            bindings.push(vk::DescriptorSetLayoutBinding {
                binding: Self::SAMPLED_IMAGE_BINDING as u32,
                descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: count.sampled_images as u32,
                stage_flags: vk::ShaderStageFlags::ALL,
                p_immutable_samplers: std::ptr::null(),
            });
            pool_sizes.push(vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: count.sampled_images as u32,
            });
        }

        if count.acceleration_structures != 0 {
            bindings.push(vk::DescriptorSetLayoutBinding {
                binding: Self::ACCELERATION_STRUCTURE_BINDING as u32,
                descriptor_type: vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
                descriptor_count: count.acceleration_structures as u32,
                stage_flags: vk::ShaderStageFlags::ALL,
                p_immutable_samplers: std::ptr::null(),
            });
            pool_sizes.push(vk::DescriptorPoolSize {
                ty: vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
                descriptor_count: count.acceleration_structures as u32,
            });
        }

        let layout = unsafe {
            device.core.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings),
                None,
            )
        }?;

        let pool = unsafe {
            device.core.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::builder()
                    .max_sets(1)
                    .pool_sizes(&pool_sizes),
                None,
            )
        }?;

        let set = unsafe {
            device.core.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(pool)
                    .set_layouts(&[layout]),
            )
        }?[0];

        Ok(Self {
            device,
            layout,
            pool,
            set,
            storage_buffer_pool: IndexPool::new(0..count.storage_buffers),
            storage_image_pool: IndexPool::new(0..count.storage_images),
            sampled_image_pool: IndexPool::new(0..count.sampled_images),
            acceleration_structure_pool: IndexPool::new(0..count.acceleration_structures),
        })
    }

    fn unbind(&mut self, binding: u16, index: u16) {
        match binding {
            Self::STORAGE_BUFFER_BINDING => self.unbind_storage_buffer(index),
            Self::STORAGE_BUFFER_BINDING => self.unbind_storage_buffer(index),
            other => panic!("Unknown binding ({})", other),
        }
    }

    fn bind_storage_buffer(&mut self, buffer: &Buffer) -> u16 {
        let index = self
            .storage_buffer_pool
            .get()
            .expect("Out of storage buffer indices");
        self.write_buffer_descriptor(
            vk::DescriptorType::STORAGE_BUFFER,
            Self::STORAGE_BUFFER_BINDING,
            index,
            &[vk::DescriptorBufferInfo {
                buffer: buffer.handle,
                offset: 0,
                range: vk::WHOLE_SIZE,
            }],
        );
        index
    }
    fn unbind_storage_buffer(&mut self, index: u16) {
        self.storage_buffer_pool.free(index);
        self.write_buffer_descriptor(
            vk::DescriptorType::STORAGE_BUFFER,
            Self::STORAGE_BUFFER_BINDING,
            index,
            &[EMPTY_BUFFER_INFO],
        );
    }

    fn bind_storage_image(&mut self, image: &Image) -> u16 {
        let index = self
            .storage_image_pool
            .get()
            .expect("Out of storage image indices");
        self.write_image_descriptor(
            vk::DescriptorType::STORAGE_IMAGE,
            Self::STORAGE_IMAGE_BINDING,
            index,
            &[vk::DescriptorImageInfo {
                sampler: vk::Sampler::null(),
                image_view: image.view,
                image_layout: vk::ImageLayout::GENERAL,
            }],
        );
        index
    }
    fn unbind_storage_image(&mut self, index: u16) {
        self.storage_image_pool.free(index);
        self.write_image_descriptor(
            vk::DescriptorType::STORAGE_IMAGE,
            Self::STORAGE_IMAGE_BINDING,
            index,
            &[EMPTY_IMAGE_INFO],
        );
    }

    fn bind_sampled_image(&mut self, image: &Image) -> u16 {
        let index = self
            .sampled_image_pool
            .get()
            .expect("Out of sampled image indices");
        self.write_image_descriptor(
            vk::DescriptorType::SAMPLED_IMAGE,
            Self::SAMPLED_IMAGE_BINDING,
            index,
            &[vk::DescriptorImageInfo {
                sampler: image.sampler.expect("Sampled image require a sampler"),
                image_view: image.view,
                image_layout: vk::ImageLayout::GENERAL, //TODO: change this to SHADER_READ_ONLY_OPTIMAL once image transitions are supported
            }],
        );
        index
    }
    fn unbind_sampled_image(&mut self, index: u16) {
        self.sampled_image_pool.free(index);
        self.write_image_descriptor(
            vk::DescriptorType::SAMPLED_IMAGE,
            Self::SAMPLED_IMAGE_BINDING,
            index,
            &[EMPTY_IMAGE_INFO],
        );
    }

    fn write_buffer_descriptor(
        &self,
        descriptor_type: vk::DescriptorType,
        binding: u16,
        index: u16,
        buffers: &[vk::DescriptorBufferInfo],
    ) {
        let descriptor_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.set)
            .dst_binding(binding as u32)
            .dst_array_element(index as u32)
            .descriptor_type(descriptor_type)
            .buffer_info(buffers);
        unsafe {
            self.device
                .core
                .update_descriptor_sets(&[descriptor_write.build()], &[]);
        }
    }

    fn write_image_descriptor(
        &self,
        descriptor_type: vk::DescriptorType,
        binding: u16,
        index: u16,
        images: &[vk::DescriptorImageInfo],
    ) {
        let descriptor_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.set)
            .dst_binding(binding as u32)
            .dst_array_element(index as u32)
            .descriptor_type(descriptor_type)
            .image_info(images);
        unsafe {
            self.device
                .core
                .update_descriptor_sets(&[descriptor_write.build()], &[]);
        }
    }

    fn write_acceleration_structure_descriptor(
        &self,
        binding: u16,
        index: u16,
        write_acceleration_structure: &mut vk::WriteDescriptorSetAccelerationStructureKHR,
    ) {
        let descriptor_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.set)
            .dst_binding(binding as u32)
            .dst_array_element(index as u32)
            .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .push_next(write_acceleration_structure);
        unsafe {
            self.device
                .core
                .update_descriptor_sets(&[descriptor_write.build()], &[]);
        }
    }
}

struct Binding {
    binding: u16,
    index: u16,
    set: Arc<Mutex<DescriptorSet>>,
}

impl Drop for Binding {
    fn drop(&mut self) {
        self.set.lock().unwrap().unbind(self.binding, self.index);
    }
}

struct IndexPool {
    range: std::ops::Range<u16>,
    freed_indices: Vec<u16>,
}

impl IndexPool {
    fn new(range: std::ops::Range<u16>) -> Self {
        Self {
            range,
            freed_indices: Vec::new(),
        }
    }

    fn get(&mut self) -> Option<u16> {
        let mut new_value = self.freed_indices.pop();
        if new_value.is_none() {
            new_value = self.range.next();
        }
        new_value
    }

    fn free(&mut self, index: u16) {
        self.freed_indices.push(index);
    }
}
