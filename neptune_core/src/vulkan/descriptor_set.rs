use crate::id_pool::IdPool;
use crate::render_backend::RenderDevice;
use crate::vulkan::{Buffer, BufferDescription, Image, ImageDescription};
use ash::vk;
use std::collections::HashMap;
use std::rc::Rc;

pub struct DescriptorSet {
    storage_buffer_indexes: IdPool,
    storage_image_indexes: IdPool,
    sampled_image_indexes: IdPool,
    sampler_indexes: IdPool,

    empty_buffer: Buffer,
    empty_image: Image,
    empty_sampler: vk::Sampler,

    //Updates
    storage_buffer_changes: HashMap<u32, vk::DescriptorBufferInfo>,
    storage_image_changes: HashMap<u32, vk::DescriptorImageInfo>,
    sampled_image_changes: HashMap<u32, vk::DescriptorImageInfo>,
    sampler_changes: HashMap<u32, vk::DescriptorImageInfo>,

    //Vulkan Objects
    device: Rc<ash::Device>,
    layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    set: vk::DescriptorSet,
}

impl DescriptorSet {
    const STORAGE_BUFFER_BINDING: u32 = 0;
    const STORAGE_IMAGE_BINDING: u32 = 2;
    const SAMPLED_IMAGE_BINDING: u32 = 3;
    const SAMPLER_BINDING: u32 = 4;

    pub(crate) fn new(
        device: RenderDevice,
        storage_buffer_count: u32,
        storage_image_count: u32,
        sampled_image_count: u32,
        sampler_count: u32,
    ) -> Self {
        let empty_buffer = Buffer::new(
            &device,
            BufferDescription {
                size: 16,
                usage: vk::BufferUsageFlags::STORAGE_BUFFER,
                memory_location: gpu_allocator::MemoryLocation::GpuOnly,
            },
        );

        let mut empty_image = Image::new(
            &device,
            ImageDescription {
                format: vk::Format::R8G8B8A8_UNORM,
                size: [16; 2],
                usage: vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
                memory_location: gpu_allocator::MemoryLocation::GpuOnly,
            },
        );
        empty_image.create_image_view();

        let empty_sampler = unsafe {
            device.base.create_sampler(
                &vk::SamplerCreateInfo::builder()
                    .mag_filter(vk::Filter::NEAREST)
                    .min_filter(vk::Filter::NEAREST)
                    .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                    .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE),
                None,
            )
        }
        .expect("Failed to create image sampler");

        let layout = unsafe {
            device.base.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder()
                    .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
                    .bindings(&[
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(Self::STORAGE_BUFFER_BINDING)
                            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                            .descriptor_count(storage_buffer_count)
                            .stage_flags(vk::ShaderStageFlags::ALL)
                            .build(),
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(Self::STORAGE_IMAGE_BINDING)
                            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                            .descriptor_count(storage_image_count)
                            .stage_flags(vk::ShaderStageFlags::ALL)
                            .build(),
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(Self::SAMPLED_IMAGE_BINDING)
                            .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                            .descriptor_count(sampled_image_count)
                            .stage_flags(vk::ShaderStageFlags::ALL)
                            .build(),
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(Self::SAMPLER_BINDING)
                            .descriptor_type(vk::DescriptorType::SAMPLER)
                            .descriptor_count(sampler_count)
                            .stage_flags(vk::ShaderStageFlags::ALL)
                            .build(),
                    ])
                    .build(),
                None,
            )
        }
        .expect("Failed to create descriptor set layout");

        let pool = unsafe {
            device.base.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::builder()
                    .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
                    .max_sets(1)
                    .pool_sizes(&[
                        vk::DescriptorPoolSize::builder()
                            .ty(vk::DescriptorType::STORAGE_BUFFER)
                            .descriptor_count(storage_buffer_count)
                            .build(),
                        vk::DescriptorPoolSize::builder()
                            .ty(vk::DescriptorType::STORAGE_IMAGE)
                            .descriptor_count(storage_image_count)
                            .build(),
                        vk::DescriptorPoolSize::builder()
                            .ty(vk::DescriptorType::SAMPLED_IMAGE)
                            .descriptor_count(sampled_image_count)
                            .build(),
                        vk::DescriptorPoolSize::builder()
                            .ty(vk::DescriptorType::SAMPLER)
                            .descriptor_count(sampler_count)
                            .build(),
                    ])
                    .build(),
                None,
            )
        }
        .expect("Failed to create descriptor pool");

        let set = unsafe {
            device.base.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(pool)
                    .set_layouts(&[layout]),
            )
        }
        .expect("Failed to allocate descriptor set")[0];

        let mut new_self = Self {
            storage_buffer_indexes: IdPool::new_with_max(0, storage_buffer_count),
            storage_image_indexes: IdPool::new_with_max(0, storage_image_count),
            sampled_image_indexes: IdPool::new_with_max(0, sampled_image_count),
            sampler_indexes: IdPool::new_with_max(0, sampler_count),
            empty_buffer,
            empty_image,
            empty_sampler,
            storage_buffer_changes: HashMap::new(),
            storage_image_changes: HashMap::new(),
            sampled_image_changes: HashMap::new(),
            sampler_changes: HashMap::new(),
            device: device.base,
            layout,
            pool,
            set,
        };
        new_self.write_empty(
            storage_buffer_count,
            storage_image_count,
            sampled_image_count,
            sampler_count,
        );
        new_self
    }

    pub(crate) fn bind_storage_buffer(&mut self, buffer: &Buffer) -> u32 {
        let index = self.storage_buffer_indexes.get();
        let _ = self.storage_buffer_changes.insert(
            index,
            vk::DescriptorBufferInfo {
                buffer: buffer.handle,
                offset: 0,
                range: buffer.description.size as vk::DeviceSize,
            },
        );
        index
    }

    pub(crate) fn bind_storage_image(&mut self, image: &Image) -> u32 {
        let index = self.storage_image_indexes.get();
        let _ = self.sampled_image_changes.insert(
            index,
            vk::DescriptorImageInfo {
                sampler: vk::Sampler::null(),
                image_view: image.view.unwrap(),
                image_layout: vk::ImageLayout::GENERAL, //Sampled images should always be in GENERAL
            },
        );
        index
    }

    pub(crate) fn bind_sampled_image(&mut self, image: &Image) -> u32 {
        let index = self.sampled_image_indexes.get();
        let _ = self.sampled_image_changes.insert(
            index,
            vk::DescriptorImageInfo {
                sampler: vk::Sampler::null(),
                image_view: image.view.unwrap(),
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, //Sampled images should always be in SHADER_READ_ONLY_OPTIMAL when read
            },
        );
        index
    }

    pub(crate) fn bind_sampler(&mut self, sampler: vk::Sampler) -> u32 {
        let index = self.sampler_indexes.get();
        let _ = self.sampler_changes.insert(
            index,
            vk::DescriptorImageInfo {
                sampler,
                image_view: vk::ImageView::null(),
                image_layout: vk::ImageLayout::UNDEFINED,
            },
        );
        index
    }

    pub(crate) fn commit_changes(&mut self) {
        let mut writes: Vec<vk::WriteDescriptorSet> = Vec::with_capacity(
            self.storage_buffer_changes.len()
                + self.storage_image_changes.len()
                + self.sampled_image_changes.len()
                + self.sampler_changes.len(),
        );

        for (&index, info) in self.storage_buffer_changes.iter() {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::STORAGE_BUFFER_BINDING,
                dst_array_element: index,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_buffer_info: info,
                ..Default::default()
            });
        }

        for (&index, info) in self.storage_image_changes.iter() {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::STORAGE_IMAGE_BINDING,
                dst_array_element: index,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                p_image_info: info,
                ..Default::default()
            });
        }

        for (&index, info) in self.sampled_image_changes.iter() {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::SAMPLED_IMAGE_BINDING,
                dst_array_element: index,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
                p_image_info: info,
                ..Default::default()
            });
        }

        for (&index, info) in self.sampler_changes.iter() {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::SAMPLER_BINDING,
                dst_array_element: index,
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

        self.storage_buffer_changes.clear();
        self.sampled_image_changes.clear();
    }

    fn write_empty(
        &mut self,
        storage_buffer_count: u32,
        storage_image_count: u32,
        sampled_image_count: u32,
        sampler_count: u32,
    ) {
        let mut writes: Vec<vk::WriteDescriptorSet> = Vec::with_capacity(
            (storage_buffer_count + storage_image_count + sampled_image_count + sampler_count)
                as usize,
        );

        let empty_buffer_info = &vk::DescriptorBufferInfo {
            buffer: self.empty_buffer.handle,
            offset: 0,
            range: vk::WHOLE_SIZE,
        };

        let empty_storage_image_info = &vk::DescriptorImageInfo {
            sampler: vk::Sampler::null(),
            image_view: self.empty_image.view.unwrap(),
            image_layout: vk::ImageLayout::GENERAL,
        };

        let empty_sampled_image_info = &vk::DescriptorImageInfo {
            sampler: vk::Sampler::null(),
            image_view: self.empty_image.view.unwrap(),
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };

        let empty_sampler_info = &vk::DescriptorImageInfo {
            sampler: self.empty_sampler,
            image_view: vk::ImageView::null(),
            image_layout: vk::ImageLayout::UNDEFINED,
        };

        for index in 0..storage_buffer_count {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::STORAGE_BUFFER_BINDING,
                dst_array_element: index,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_buffer_info: empty_buffer_info,
                ..Default::default()
            });
        }

        for index in 0..storage_image_count {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::STORAGE_IMAGE_BINDING,
                dst_array_element: index,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                p_image_info: empty_storage_image_info,
                ..Default::default()
            });
        }

        for index in 0..sampled_image_count {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::SAMPLED_IMAGE_BINDING,
                dst_array_element: index,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
                p_image_info: empty_sampled_image_info,
                ..Default::default()
            });
        }

        for index in 0..sampler_count {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::SAMPLER_BINDING,
                dst_array_element: index,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::SAMPLER,
                p_image_info: empty_sampler_info,
                ..Default::default()
            });
        }

        //TODO: profile this part
        unsafe {
            self.device.update_descriptor_sets(&writes, &[]);
        }
    }
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_pool(self.pool, None);
            self.device.destroy_descriptor_set_layout(self.layout, None);
            self.device.destroy_sampler(self.empty_sampler, None);
        }
    }
}