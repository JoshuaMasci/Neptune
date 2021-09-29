use crate::buffer::Buffer;
use crate::id_pool::IdPool;
use crate::image::Image;
use ash::*;

pub(crate) struct DescriptorSetManager {
    device: ash::Device,
    descriptor_set: DescriptorSet,

    storage_buffer_indices: IdPool,
    storage_image_indices: IdPool,
    sampled_image_indices: IdPool,
    sampler_indices: IdPool,
    acceleration_structure_indices: IdPool,
}

impl DescriptorSetManager {
    pub(crate) fn new(device: &ash::Device, counts: &DescriptorCount) -> Self {
        let descriptor_set = DescriptorSet::new(device, counts);
        let device = device.clone();

        Self {
            device,
            descriptor_set,
            storage_buffer_indices: IdPool::new_with_max(0, counts.storage_buffer),
            storage_image_indices: IdPool::new_with_max(0, counts.storage_image),
            sampled_image_indices: IdPool::new_with_max(0, counts.sampled_image),
            sampler_indices: IdPool::new_with_max(0, counts.sampler),
            acceleration_structure_indices: IdPool::new_with_max(0, counts.acceleration_structure),
        }
    }

    pub(crate) fn bind_storage_buffer(&mut self, buffer: &Buffer) -> u32 {
        let index = self.storage_buffer_indices.get();
        self.descriptor_set.storage_buffer_changes.0.push(index);
        self.descriptor_set.storage_buffer_changes.1.push(
            vk::DescriptorBufferInfo::builder()
                .buffer(buffer.buffer)
                .offset(0)
                .range(buffer.size)
                .build(),
        );
        index
    }

    pub(crate) fn unbind_storage_buffer(&mut self, index: u32) {
        self.descriptor_set.storage_buffer_changes.0.push(index);
        self.descriptor_set.storage_buffer_changes.1.push(
            vk::DescriptorBufferInfo::builder()
                .buffer(vk::Buffer::null())
                .offset(0)
                .range(vk::WHOLE_SIZE)
                .build(),
        );
        self.storage_buffer_indices.free(index);
    }

    pub(crate) fn bind_storage_image(&mut self, image: &Image, layout: vk::ImageLayout) -> u32 {
        let index = self.storage_image_indices.get();
        self.descriptor_set.storage_image_changes.0.push(index);
        self.descriptor_set.storage_image_changes.1.push(
            vk::DescriptorImageInfo::builder()
                .image_view(image.get_image_view())
                .image_layout(layout)
                .build(),
        );
        index
    }

    pub(crate) fn unbind_storage_image(&mut self, index: u32) {
        self.descriptor_set.storage_image_changes.0.push(index);
        self.descriptor_set.storage_image_changes.1.push(
            vk::DescriptorImageInfo::builder()
                .image_view(vk::ImageView::null())
                .image_layout(vk::ImageLayout::UNDEFINED)
                .build(),
        );
        self.storage_image_indices.free(index);
    }

    pub(crate) fn bind_sampled_image(&mut self, image: &Image, layout: vk::ImageLayout) -> u32 {
        let index = self.sampled_image_indices.get();
        self.descriptor_set.sampled_image_changes.0.push(index);
        self.descriptor_set.sampled_image_changes.1.push(
            vk::DescriptorImageInfo::builder()
                .image_view(image.get_image_view())
                .image_layout(layout)
                .build(),
        );
        index
    }

    pub(crate) fn unbind_sampled_image(&mut self, index: u32) {
        self.descriptor_set.sampled_image_changes.0.push(index);
        self.descriptor_set.sampled_image_changes.1.push(
            vk::DescriptorImageInfo::builder()
                .image_view(vk::ImageView::null())
                .image_layout(vk::ImageLayout::UNDEFINED)
                .build(),
        );
        self.sampled_image_indices.free(index);
    }

    pub(crate) fn commit_changes(&mut self) {
        self.descriptor_set.commit_changes();
    }
}

pub(crate) struct DescriptorCount {
    pub(crate) storage_buffer: u32,
    pub(crate) storage_image: u32,
    pub(crate) sampled_image: u32,
    pub(crate) sampler: u32,
    pub(crate) acceleration_structure: u32,
}

pub(crate) struct DescriptorSet {
    device: ash::Device,
    layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    set: vk::DescriptorSet,

    storage_buffer_changes: (Vec<u32>, Vec<vk::DescriptorBufferInfo>),
    storage_image_changes: (Vec<u32>, Vec<vk::DescriptorImageInfo>),
    sampled_image_changes: (Vec<u32>, Vec<vk::DescriptorImageInfo>),
}

impl DescriptorSet {
    const STORAGE_BUFFER_BINDING: u32 = 0;
    const STORAGE_IMAGE_BINDING: u32 = 1;
    const SAMPLED_IMAGE_BINDING: u32 = 2;
    const SAMPLER_BINDING: u32 = 3;
    const ACCELERATION_STRUCTURE_BINDING: u32 = 4;

    const DESCRIPTOR_TYPES: [vk::DescriptorType; 5] = [
        vk::DescriptorType::STORAGE_BUFFER,
        vk::DescriptorType::STORAGE_IMAGE,
        vk::DescriptorType::SAMPLED_IMAGE,
        vk::DescriptorType::SAMPLER,
        vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
    ];

    pub(crate) fn new(device: &ash::Device, counts: &DescriptorCount) -> Self {
        let device = device.clone();

        let binding_counts = &[
            counts.storage_buffer,
            counts.storage_image,
            counts.sampled_image,
            counts.sampler,
            counts.acceleration_structure,
        ];

        let bindings: Vec<vk::DescriptorSetLayoutBinding> = binding_counts
            .iter()
            .enumerate()
            .filter(|(_index, &count)| count > 0)
            .map(|(index, &count)| {
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(index as u32)
                    .descriptor_type(DescriptorSet::DESCRIPTOR_TYPES[index])
                    .descriptor_count(count)
                    .stage_flags(vk::ShaderStageFlags::all())
                    .build()
            })
            .collect();

        let pool_sizes: Vec<vk::DescriptorPoolSize> = binding_counts
            .iter()
            .enumerate()
            .filter(|(_index, &count)| count > 0)
            .map(|(index, &count)| {
                vk::DescriptorPoolSize::builder()
                    .ty(DescriptorSet::DESCRIPTOR_TYPES[index])
                    .descriptor_count(count)
                    .build()
            })
            .collect();

        let layout = unsafe {
            device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder()
                    .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
                    .bindings(&bindings)
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
                    .pool_sizes(&pool_sizes)
                    .build(),
                None,
            )
        }
        .expect("Failed to create descriptor pool");

        let sets = unsafe {
            device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(pool)
                    .set_layouts(&[layout]),
            )
        }
        .expect("Failed to allocate descriptor sets");

        Self {
            device,
            layout,
            pool,
            set: sets[0],
            storage_buffer_changes: (Vec::new(), Vec::new()),
            storage_image_changes: (Vec::new(), Vec::new()),
            sampled_image_changes: (Vec::new(), Vec::new()),
        }
    }

    fn commit_changes(&mut self) {
        let write_counts = self.storage_buffer_changes.0.len()
            + self.storage_image_changes.0.len()
            + self.sampled_image_changes.0.len();
        let mut writes: Vec<vk::WriteDescriptorSet> = Vec::with_capacity(write_counts);

        for (&index, info) in self
            .storage_buffer_changes
            .0
            .iter()
            .zip(self.storage_buffer_changes.1.iter())
        {
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

        for (&index, info) in self
            .storage_image_changes
            .0
            .iter()
            .zip(self.storage_image_changes.1.iter())
        {
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

        for (&index, info) in self
            .sampled_image_changes
            .0
            .iter()
            .zip(self.sampled_image_changes.1.iter())
        {
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

        //TODO: profile this part
        unsafe {
            self.device.update_descriptor_sets(&writes, &[]);
        }

        self.storage_buffer_changes.0.clear();
        self.storage_image_changes.0.clear();
        self.sampled_image_changes.0.clear();

        self.storage_buffer_changes.1.clear();
        self.storage_image_changes.1.clear();
        self.sampled_image_changes.1.clear();
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
