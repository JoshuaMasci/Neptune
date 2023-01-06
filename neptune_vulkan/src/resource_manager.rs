use crate::buffer::AshBuffer;
use crate::compute_pipeline::AshComputePipeline;
use crate::debug_utils::DebugUtils;
use crate::descriptor_set::{BindingCount, DescriptorSet};
use crate::sampler::AshSampler;
use crate::texture::AshImage;
use crate::{AshDevice, BufferUsage, Sampler, SamplerCreateInfo, TextureUsage};
use ash::vk;
use gpu_allocator::MemoryLocation;
use slotmap::SlotMap;
use std::sync::{Arc, Mutex};

pub(crate) struct BufferResource {
    buffer: AshBuffer,
    uniform_binding: Option<u32>,
    storage_binding: Option<u32>,
}

pub(crate) struct TextureResource {
    texture: AshImage,
    sampled_binding: Option<(u32, Arc<AshSampler>)>,
    storage_binding: Option<u32>,
}

slotmap::new_key_type! {
    pub struct SwapchainHandle;
    pub struct BufferHandle;
    pub struct TextureHandle;
    pub struct SamplerHandle;
    pub struct ComputePipelineHandle;
    pub struct RasterPipelineHandle;
}

pub(crate) struct ResourceManager {
    debug_utils: Option<Arc<DebugUtils>>,
    device: Arc<AshDevice>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    descriptor_set: DescriptorSet,

    buffers: SlotMap<BufferHandle, BufferResource>,
    textures: SlotMap<TextureHandle, TextureResource>,
    samplers: SlotMap<SamplerHandle, Arc<AshSampler>>,
    compute_pipelines: SlotMap<ComputePipelineHandle, Arc<AshComputePipeline>>,
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
            buffers: SlotMap::with_key(),
            textures: SlotMap::with_key(),
            samplers: SlotMap::with_key(),
            compute_pipelines: SlotMap::with_key(),
        })
    }

    pub(crate) fn update(&mut self) {
        self.descriptor_set.update();
    }

    pub(crate) fn create_buffer(
        &mut self,
        name: &str,
        usage: BufferUsage,
        size: u64,
    ) -> crate::Result<BufferHandle> {
        let buffer = crate::buffer::AshBuffer::new(
            self.device.clone(),
            self.allocator.clone(),
            &crate::buffer::get_vk_buffer_create_info(usage, size),
            MemoryLocation::GpuOnly,
        )?;
        self.set_debug_name(buffer.handle, name);

        let uniform_binding = if usage.contains(BufferUsage::UNIFORM) {
            Some(self.descriptor_set.bind_uniform_buffer(&buffer)?)
        } else {
            None
        };

        let storage_binding = if usage.contains(BufferUsage::STORAGE) {
            Some(self.descriptor_set.bind_storage_buffer(&buffer)?)
        } else {
            None
        };

        let resource = BufferResource {
            buffer,
            uniform_binding,
            storage_binding,
        };

        Ok(self.buffers.insert(resource))
    }

    pub(crate) fn destroy_buffer(&mut self, handle: BufferHandle) {
        //Drop Immediately for now
        if let Some(resource) = self.buffers.remove(handle) {
            if let Some(binding) = resource.uniform_binding {
                self.descriptor_set.unbind_uniform_buffer(binding)
            }
            if let Some(binding) = resource.storage_binding {
                self.descriptor_set.unbind_storage_buffer(binding)
            }
        }
    }

    pub fn create_texture(
        &mut self,
        name: &str,
        usage: TextureUsage,
        format: vk::Format,
        size: [u32; 2],
        sampler: Option<&Sampler>,
    ) -> crate::Result<TextureHandle> {
        let sampler = if let Some(sampler) = sampler {
            Some(self.samplers.get(sampler.handle).unwrap().clone())
        } else {
            None
        };

        let is_sampled = sampler
            .as_ref()
            .map(|_| TextureUsage::SAMPLED)
            .unwrap_or(TextureUsage::empty());
        let texture = crate::texture::AshImage::new(
            self.device.clone(),
            self.allocator.clone(),
            usage | is_sampled,
            format,
            size,
            MemoryLocation::GpuOnly,
        )?;
        self.set_debug_name(texture.handle, name);

        let storage_binding = if usage.contains(TextureUsage::STORAGE) {
            Some(self.descriptor_set.bind_storage_image(&texture)?)
        } else {
            None
        };

        let sampled_binding = if let Some(sampler) = sampler {
            Some((
                self.descriptor_set.bind_sampled_image(&texture, &sampler)?,
                sampler,
            ))
        } else {
            None
        };

        let resource = TextureResource {
            texture,
            sampled_binding,
            storage_binding,
        };

        Ok(self.textures.insert(resource))
    }

    pub(crate) fn destroy_texture(&mut self, handle: TextureHandle) {
        //Drop Immediately for now
        if let Some(resource) = self.textures.remove(handle) {
            if let Some((binding, _)) = resource.sampled_binding {
                self.descriptor_set.unbind_sampled_image(binding)
            }

            if let Some(binding) = resource.storage_binding {
                self.descriptor_set.unbind_storage_image(binding)
            }
        }
    }

    pub(crate) fn create_sampler(
        &mut self,
        name: &str,
        sampler_create_info: &SamplerCreateInfo,
    ) -> crate::Result<SamplerHandle> {
        let sampler = AshSampler::new(self.device.clone(), sampler_create_info)?;
        self.set_debug_name(sampler.handle, name);
        Ok(self.samplers.insert(Arc::new(sampler)))
    }

    pub(crate) fn destroy_sampler(&mut self, handle: SamplerHandle) {
        //Drop Immediately, The Arc will handle the remaining lifetime
        let _ = self.samplers.remove(handle);
    }

    pub(crate) fn create_compute_pipeline(
        &mut self,
        name: &str,
        code: &[u32],
    ) -> crate::Result<ComputePipelineHandle> {
        let compute_pipeline = AshComputePipeline::new(
            self.device.clone(),
            vk::PipelineCache::null(),
            vk::PipelineLayout::null(),
            code,
        )?;
        self.set_debug_name(compute_pipeline.handle, name);
        Ok(self.compute_pipelines.insert(Arc::new(compute_pipeline)))
    }

    pub(crate) fn destroy_compute_pipeline(&mut self, handle: ComputePipelineHandle) {
        //Drop Immediately, The Arc will handle the remaining lifetime
        let _ = self.compute_pipelines.remove(handle);
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
