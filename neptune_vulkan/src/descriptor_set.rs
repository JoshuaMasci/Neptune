use crate::buffer::Buffer;
use crate::device::AshDevice;
use crate::image::Image;
use crate::{Sampler, VulkanError};
use ash::vk;
use std::sync::{Arc, Mutex};

#[derive(Default, Debug, Clone)]
pub struct DescriptorCount {
    pub storage_buffers: u16,
    pub storage_images: u16,
    pub sampled_images: u16,
    pub samplers: u16,
    pub acceleration_structures: u16,
}

pub struct DescriptorBinding {
    binding: u16,
    index: u16,
    set: Arc<Mutex<DescriptorSetInner>>,
}

impl DescriptorBinding {
    pub(crate) fn index(&self) -> u32 {
        //TODO: Use last 16(?) bits of index to encode the binding for GPU error checking
        self.index as u32
    }
}

impl Drop for DescriptorBinding {
    fn drop(&mut self) {
        self.set.lock().unwrap().unbind(self.binding, self.index);
    }
}

pub struct DescriptorSet {
    inner: Arc<Mutex<DescriptorSetInner>>,
    layout: vk::DescriptorSetLayout,
    set: vk::DescriptorSet,
}

impl DescriptorSet {
    pub fn new(device: Arc<AshDevice>, count: DescriptorCount) -> Result<Self, VulkanError> {
        let inner = DescriptorSetInner::new(device, count)?;
        let layout = inner.layout;
        let set = inner.set;
        let inner = Arc::new(Mutex::new(inner));

        Ok(Self { inner, layout, set })
    }

    pub fn get_layout(&self) -> vk::DescriptorSetLayout {
        self.layout
    }

    pub fn get_set(&self) -> vk::DescriptorSet {
        self.set
    }

    pub fn bind_storage_buffer(&self, buffer: &Buffer) -> DescriptorBinding {
        DescriptorBinding {
            binding: DescriptorSetInner::STORAGE_BUFFER_BINDING,
            index: self.inner.lock().unwrap().bind_storage_buffer(buffer),
            set: self.inner.clone(),
        }
    }

    pub fn bind_storage_image(&self, image: &Image) -> DescriptorBinding {
        DescriptorBinding {
            binding: DescriptorSetInner::STORAGE_IMAGE_BINDING,
            index: self.inner.lock().unwrap().bind_storage_image(image),
            set: self.inner.clone(),
        }
    }

    pub fn bind_sampled_image(&self, image: &Image) -> DescriptorBinding {
        DescriptorBinding {
            binding: DescriptorSetInner::SAMPLED_IMAGE_BINDING,
            index: self.inner.lock().unwrap().bind_sampled_image(image),
            set: self.inner.clone(),
        }
    }

    pub fn bind_sampler(&self, sampler: &Sampler) -> DescriptorBinding {
        DescriptorBinding {
            binding: DescriptorSetInner::SAMPLER_BINDING,
            index: self.inner.lock().unwrap().bind_sampler(sampler),
            set: self.inner.clone(),
        }
    }
}

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

pub struct DescriptorSetInner {
    device: Arc<AshDevice>,
    layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    set: vk::DescriptorSet,
    empty_sampler: vk::Sampler,

    storage_buffer_pool: IndexPool,
    storage_image_pool: IndexPool,
    sampled_image_pool: IndexPool,
    sampler_pool: IndexPool,
    //acceleration_structure_pool: IndexPool,
}

impl DescriptorSetInner {
    //TODO: separate sampled images and samplers
    const STORAGE_BUFFER_BINDING: u16 = 0;
    const STORAGE_IMAGE_BINDING: u16 = 1;
    const SAMPLED_IMAGE_BINDING: u16 = 2;
    const SAMPLER_BINDING: u16 = 3;
    const ACCELERATION_STRUCTURE_BINDING: u16 = 4;

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

        if count.samplers != 0 {
            bindings.push(vk::DescriptorSetLayoutBinding {
                binding: Self::SAMPLER_BINDING as u32,
                descriptor_type: vk::DescriptorType::SAMPLER,
                descriptor_count: count.samplers as u32,
                stage_flags: vk::ShaderStageFlags::ALL,
                p_immutable_samplers: std::ptr::null(),
            });
            pool_sizes.push(vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLER,
                descriptor_count: count.samplers as u32,
            });
        }

        let _ = count.acceleration_structures;
        // if count.acceleration_structures != 0 {
        //     bindings.push(vk::DescriptorSetLayoutBinding {
        //         binding: Self::ACCELERATION_STRUCTURE_BINDING as u32,
        //         descriptor_type: vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
        //         descriptor_count: count.acceleration_structures as u32,
        //         stage_flags: vk::ShaderStageFlags::ALL,
        //         p_immutable_samplers: std::ptr::null(),
        //     });
        //     pool_sizes.push(vk::DescriptorPoolSize {
        //         ty: vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
        //         descriptor_count: count.acceleration_structures as u32,
        //     });
        // }

        let binding_flags = vec![
            vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
                | vk::DescriptorBindingFlags::PARTIALLY_BOUND
                | vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING;
            bindings.len()
        ];
        let mut binding_flag_create_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
            .binding_flags(&binding_flags)
            .build();

        let layout = unsafe {
            device.core.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder()
                    .bindings(&bindings)
                    .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
                    .push_next(&mut binding_flag_create_info),
                None,
            )
        }?;

        let pool = unsafe {
            device.core.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::builder()
                    .max_sets(1)
                    .pool_sizes(&pool_sizes)
                    .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND),
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

        let empty_sampler = unsafe {
            device
                .core
                .create_sampler(&vk::SamplerCreateInfo::default(), None)?
        };

        let new_self = Self {
            device,
            layout,
            pool,
            set,
            empty_sampler,
            storage_buffer_pool: IndexPool::new(0..count.storage_buffers),
            storage_image_pool: IndexPool::new(0..count.storage_images),
            sampled_image_pool: IndexPool::new(0..count.sampled_images),
            sampler_pool: IndexPool::new(0..count.samplers),
            //acceleration_structure_pool: IndexPool::new(0..count.acceleration_structures),
        };

        //Write empty sampler
        {
            let sampler_info = vk::DescriptorImageInfo {
                sampler: empty_sampler,
                image_view: vk::ImageView::null(),
                image_layout: vk::ImageLayout::UNDEFINED,
            };
            let default_samplers = vec![sampler_info; count.samplers as usize];
            new_self.write_image_descriptor(
                vk::DescriptorType::SAMPLER,
                Self::SAMPLER_BINDING,
                0,
                &default_samplers,
            );
        }

        Ok(new_self)
    }

    fn unbind(&mut self, binding: u16, index: u16) {
        match binding {
            Self::STORAGE_BUFFER_BINDING => self.unbind_storage_buffer(index),
            Self::STORAGE_IMAGE_BINDING => self.unbind_storage_image(index),
            Self::SAMPLED_IMAGE_BINDING => self.unbind_sampled_image(index),
            Self::SAMPLER_BINDING => self.unbind_sampler(index),
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
                sampler: vk::Sampler::null(),
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

    fn bind_sampler(&mut self, sampler: &Sampler) -> u16 {
        let index = self.sampler_pool.get().expect("Out of sampler indices");

        self.write_image_descriptor(
            vk::DescriptorType::SAMPLER,
            Self::SAMPLER_BINDING,
            index,
            &[vk::DescriptorImageInfo {
                sampler: sampler.handle,
                image_view: vk::ImageView::null(),
                image_layout: vk::ImageLayout::UNDEFINED,
            }],
        );
        index
    }
    fn unbind_sampler(&mut self, index: u16) {
        self.sampler_pool.free(index);
        self.write_image_descriptor(
            vk::DescriptorType::SAMPLER,
            Self::SAMPLER_BINDING,
            index,
            &[vk::DescriptorImageInfo {
                sampler: self.empty_sampler,
                image_view: vk::ImageView::null(),
                image_layout: vk::ImageLayout::UNDEFINED,
            }],
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

impl Drop for DescriptorSetInner {
    fn drop(&mut self) {
        unsafe {
            self.device.core.destroy_sampler(self.empty_sampler, None);

            //No need to free the descriptor set, it will be destroyed up with the pool
            self.device.core.destroy_descriptor_pool(self.pool, None);
            self.device
                .core
                .destroy_descriptor_set_layout(self.layout, None);
        }
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
