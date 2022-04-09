use crate::id_pool::IdPool;
use crate::vulkan::buffer::Buffer;
use crate::vulkan::texture::Texture;
use crate::BufferDescription;
use ash::vk;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Binding {
    pub index: u32,
    pub binding_type: BindingType,
    freed_bindings: Rc<RefCell<Vec<(u32, BindingType)>>>,
}

impl Drop for Binding {
    fn drop(&mut self) {
        let mut list = (*self.freed_bindings).borrow_mut();
        list.push((self.index, self.binding_type));
    }
}

#[derive(Copy, Clone)]
pub enum BindingType {
    StorageBuffer,
    StorageImage,
    SampledImage,
    Sampler,
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

pub struct DescriptorSet {
    storage_buffer_indexes: IdPool,
    storage_image_indexes: IdPool,
    sampled_image_indexes: IdPool,
    sampler_indexes: IdPool,

    //Updates
    storage_buffer_changes: HashMap<u32, vk::DescriptorBufferInfo>,
    storage_image_changes: HashMap<u32, vk::DescriptorImageInfo>,
    sampled_image_changes: HashMap<u32, vk::DescriptorImageInfo>,
    sampler_changes: HashMap<u32, vk::DescriptorImageInfo>,
    freed_bindings: Rc<RefCell<Vec<(u32, BindingType)>>>,

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
        device: Rc<ash::Device>,
        storage_buffer_count: u32,
        storage_image_count: u32,
        sampled_image_count: u32,
        sampler_count: u32,
    ) -> Self {
        let binding_flags = [vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            | vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING;
            4];
        let mut binding_flag_create_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
            .binding_flags(&binding_flags)
            .build();

        let layout = unsafe {
            device.create_descriptor_set_layout(
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
                    .push_next(&mut binding_flag_create_info)
                    .build(),
                None,
            )
        }
        .expect("Failed to create descriptor set layout");

        let pool = unsafe {
            device.create_descriptor_pool(
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
            device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(pool)
                    .set_layouts(&[layout]),
            )
        }
        .expect("Failed to allocate descriptor set")[0];

        Self {
            storage_buffer_indexes: IdPool::new_with_max(0, storage_buffer_count),
            storage_image_indexes: IdPool::new_with_max(0, storage_image_count),
            sampled_image_indexes: IdPool::new_with_max(0, sampled_image_count),
            sampler_indexes: IdPool::new_with_max(0, sampler_count),
            storage_buffer_changes: HashMap::new(),
            storage_image_changes: HashMap::new(),
            sampled_image_changes: HashMap::new(),
            sampler_changes: HashMap::new(),
            freed_bindings: Rc::new(RefCell::new(Vec::new())),
            device,
            layout,
            pool,
            set,
        }
        //TODO: figure out if I need to write null or not
        // new_self.write_empty(
        //     storage_buffer_count,
        //     storage_image_count,
        //     sampled_image_count,
        //     sampler_count,
        // );
        //new_self
    }

    pub(crate) fn get_layout(&self) -> vk::DescriptorSetLayout {
        self.layout
    }

    pub(crate) fn get_set(&self) -> vk::DescriptorSet {
        self.set
    }

    pub(crate) fn bind_storage_buffer(&mut self, buffer: &Buffer) -> Binding {
        let index = self.storage_buffer_indexes.get();
        let _ = self.storage_buffer_changes.insert(
            index,
            vk::DescriptorBufferInfo {
                buffer: buffer.handle,
                offset: 0,
                range: buffer.description.size as vk::DeviceSize,
            },
        );
        Binding {
            index,
            binding_type: BindingType::StorageBuffer,
            freed_bindings: self.freed_bindings.clone(),
        }
    }

    pub(crate) fn bind_storage_image(&mut self, texture: &Texture) -> Binding {
        let index = self.storage_image_indexes.get();
        let _ = self.sampled_image_changes.insert(
            index,
            vk::DescriptorImageInfo {
                sampler: vk::Sampler::null(),
                image_view: texture.view,
                image_layout: vk::ImageLayout::GENERAL, //Sampled images should always be in GENERAL
            },
        );
        Binding {
            index,
            binding_type: BindingType::StorageImage,
            freed_bindings: self.freed_bindings.clone(),
        }
    }

    pub(crate) fn bind_sampled_image(&mut self, texture: &Texture) -> Binding {
        let index = self.sampled_image_indexes.get();
        let _ = self.sampled_image_changes.insert(
            index,
            vk::DescriptorImageInfo {
                sampler: vk::Sampler::null(),
                image_view: texture.view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, //Sampled images should always be in SHADER_READ_ONLY_OPTIMAL when read
            },
        );
        Binding {
            index,
            binding_type: BindingType::SampledImage,
            freed_bindings: self.freed_bindings.clone(),
        }
    }
    //
    // pub(crate) fn bind_sampler(&mut self, sampler: vk::Sampler) -> u32 {
    //     let index = self.sampler_indexes.get();
    //     let _ = self.sampler_changes.insert(
    //         index,
    //         vk::DescriptorImageInfo {
    //             sampler,
    //             image_view: vk::ImageView::null(),
    //             image_layout: vk::ImageLayout::UNDEFINED,
    //         },
    //     );
    //     index
    // }

    pub(crate) fn commit_changes(&mut self) {
        {
            let mut list = (*self.freed_bindings).borrow_mut();
            for (binding, binding_type) in list.drain(..) {
                match binding_type {
                    BindingType::StorageBuffer => {
                        self.storage_buffer_indexes.free(binding);
                        self.storage_buffer_changes
                            .insert(binding, EMPTY_BUFFER_INFO);
                    }
                    BindingType::StorageImage => {
                        self.storage_image_indexes.free(binding);
                        self.storage_image_changes.insert(binding, EMPTY_IMAGE_INFO);
                    }
                    BindingType::SampledImage => {
                        self.sampled_image_indexes.free(binding);
                        self.sampled_image_changes.insert(binding, EMPTY_IMAGE_INFO);
                    }
                    BindingType::Sampler => {
                        self.sampler_indexes.free(binding);
                        self.sampler_changes.insert(binding, EMPTY_IMAGE_INFO);
                    }
                }
            }
        }

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

        for index in 0..storage_buffer_count {
            writes.push(vk::WriteDescriptorSet {
                dst_set: self.set,
                dst_binding: Self::STORAGE_BUFFER_BINDING,
                dst_array_element: index,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_buffer_info: &EMPTY_BUFFER_INFO,
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
                p_image_info: &EMPTY_IMAGE_INFO,
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
                p_image_info: &EMPTY_IMAGE_INFO,
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
                p_image_info: &EMPTY_IMAGE_INFO,
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
        }
    }
}
