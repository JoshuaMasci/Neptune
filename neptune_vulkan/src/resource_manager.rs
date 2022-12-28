use crate::buffer::AshBuffer;
use crate::debug_utils::DebugUtils;
use crate::descriptor_set::{BindingCount, DescriptorSet};
use crate::sampler::AshSampler;
use crate::texture::AshImage;
use crate::{
    AshDevice, BufferBindingType, BufferUsage, SamplerCreateInfo, TextureBindingType, TextureUsage,
};
use ash::vk;
use gpu_allocator::MemoryLocation;
use neptune_core::IndexPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// pub(crate) struct RangeCycle {
//     range: std::ops::Range<usize>,
//     index: usize,
// }
//
// impl RangeCycle {
//     pub(crate) fn new(range: std::ops::Range<usize>) -> Self {
//         let index = range.start;
//         Self { range, index }
//     }
//
//     pub(crate) fn get(&self) -> usize {
//         self.index
//     }
//
//     pub(crate) fn increment(&mut self) {
//         let mut new_index = self.index + 1;
//         if new_index == self.range.end {
//             new_index = self.range.start;
//         }
//         self.index = new_index;
//     }
//
//     pub(crate) fn get_previous(&self, steps: usize) -> usize {
//         let mut last_index = self.index;
//         for _ in 0..steps {
//             last_index = (if last_index == self.range.start {
//                 self.range.end
//             } else {
//                 last_index
//             } - 1);
//         }
//         last_index
//     }
// }
//
// #[derive(Copy, Clone)]
// pub(crate) enum BufferAccessType {
//     Some,
//     Other,
// }
//
// #[derive(Copy, Clone)]
// pub(crate) enum TextureAccessType {
//     Some,
//     Other,
// }
//
// #[derive(Default)]
// struct ResourceFrame {
//     buffer_usages: HashMap<vk::Buffer, BufferAccessType>,
//     texture_usage: HashMap<vk::Image, TextureAccessType>,
// }
//
// impl ResourceFrame {
//     fn clear(&mut self) {
//         self.buffer_usages.clear();
//         self.texture_usage.clear();
//     }
// }

#[repr(transparent)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct BufferHandle(u16);

#[repr(transparent)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct TextureHandle(u16);

#[repr(transparent)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SamplerHandle(u16);

pub(crate) struct ResourceManager {
    debug_utils: Option<Arc<DebugUtils>>,
    device: Arc<AshDevice>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    descriptor_set: DescriptorSet,

    buffer_index_pool: IndexPool<u16>,
    buffers: HashMap<BufferHandle, AshBuffer>,

    texture_index_pool: IndexPool<u16>,
    textures: HashMap<TextureHandle, AshImage>,

    samplers: HashMap<SamplerHandle, AshSampler>,
    // frames: Vec<ResourceFrame>,
    // current_frame: RangeCycle,
}

impl ResourceManager {
    pub(crate) fn new(
        frames_in_flight_count: usize,
        device: Arc<AshDevice>,
        allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
        debug_utils: Option<Arc<DebugUtils>>,
    ) -> crate::Result<Self> {
        let _ = frames_in_flight_count;

        //TODO: allow user to decide this amount
        let descriptor_set = DescriptorSet::new(
            device.clone(),
            BindingCount {
                uniform_buffers: 4096,
                storage_buffers: 4096,
                sampled_textures: 4096,
                storage_textures: 4096,
                samplers: 256,
                acceleration_structures: 0,
            },
        )?;

        if let Some(debug_utils) = &debug_utils {
            debug_utils.set_object_name(
                device.handle(),
                descriptor_set.set(),
                "Bindless-Descriptor-Set",
            );
        }

        Ok(Self {
            debug_utils,
            device,
            allocator,
            descriptor_set,
            buffer_index_pool: IndexPool::new(0),
            buffers: HashMap::new(),
            texture_index_pool: IndexPool::new(0),
            textures: HashMap::new(),
            samplers: HashMap::new(),
        })
    }

    pub(crate) fn update(&mut self) {
        self.descriptor_set.update();
    }

    pub(crate) fn create_buffer(
        &mut self,
        name: &str,
        usage: BufferUsage,
        binding: BufferBindingType,
        size: u64,
    ) -> crate::Result<BufferHandle> {
        let buffer = crate::buffer::AshBuffer::new(
            &self.device,
            &self.allocator,
            &crate::buffer::get_vk_buffer_create_info(usage, binding, size),
            MemoryLocation::GpuOnly,
        )?;
        self.set_debug_name(buffer.handle, name);

        let handle = BufferHandle(self.buffer_index_pool.get().unwrap());

        //TODO: Bindings

        self.buffers.insert(handle, buffer);
        Ok(handle)
    }

    pub(crate) fn destroy_buffer(&mut self, handle: BufferHandle) {
        //Drop Immediately for now
        if let Some(mut buffer) = self.buffers.remove(&handle) {
            //TODO: Bindings
            buffer.destroy(&self.device, &self.allocator);
        }
    }

    pub fn create_texture(
        &mut self,
        name: &str,
        usage: TextureUsage,
        bindings: TextureBindingType,
        format: vk::Format,
        size: [u32; 2],
    ) -> crate::Result<TextureHandle> {
        let texture = crate::texture::AshImage::new(
            &self.device,
            &self.allocator,
            &crate::texture::get_vk_texture_2d_create_info(usage, bindings, format, size),
            MemoryLocation::GpuOnly,
        )?;
        self.set_debug_name(texture.handle, name);

        let handle = TextureHandle(self.texture_index_pool.get().unwrap());

        //TODO: Bindings

        self.textures.insert(handle, texture);
        Ok(handle)
    }

    pub(crate) fn destroy_texture(&mut self, handle: TextureHandle) {
        //Drop Immediately for now
        if let Some(mut texture) = self.textures.remove(&handle) {
            //TODO: Bindings
            texture.destroy(&self.device, &self.allocator);
        }
    }

    pub(crate) fn create_sampler(
        &mut self,
        name: &str,
        sampler_create_info: &SamplerCreateInfo,
    ) -> crate::Result<SamplerHandle> {
        let sampler = AshSampler::new(&self.device, sampler_create_info)?;
        self.set_debug_name(sampler.handle, name);
        let binding = SamplerHandle(self.descriptor_set.bind_sampler(sampler.handle)? as u16);
        self.samplers.insert(binding, sampler);
        Ok(binding)
    }

    pub(crate) fn destroy_sampler(&mut self, handle: SamplerHandle) {
        //Drop Immediately for now
        if let Some(sampler) = self.samplers.remove(&handle) {
            self.descriptor_set.unbind_sampler(handle.0 as u32);
            sampler.destroy(&self.device);
        }
    }

    pub(crate) fn set_debug_name<T: vk::Handle>(&self, object: T, name: &str) {
        if let Some(debug_utils) = &self.debug_utils {
            debug_utils.set_object_name(self.device.handle(), object, name);
        }
    }
}

impl Drop for ResourceManager {
    fn drop(&mut self) {}
}
