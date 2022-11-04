use crate::{AshDevice, Buffer, Image};
pub use ash::vk;
use std::sync::Arc;

pub type SamplerBinding = Arc<u32>; //TODO: replace with real type
pub type CombinedImageSamplerBinding = (SampledImageBinding, SamplerBinding);
pub type SampledImageBinding = Arc<Image>;
pub type StorageImageBinding = Arc<Image>;
pub type UniformBufferBinding = Arc<Buffer>;
pub type StorageBufferBinding = Arc<Buffer>;
pub type UniformBufferDynamicBinding = Arc<Buffer>;
pub type StorageBufferDynamicBinding = Arc<Buffer>;
pub type AccelerationStructureBinding = Arc<u32>; //TODO: replace with real type

#[derive(Copy, Clone, Hash, Debug)]
pub enum DescriptorType {
    Sampler,
    CombinedImageSampler,
    SampledImage,
    StorageImage,
    UniformBuffer,
    StorageBuffer,
    UniformBufferDynamic,
    StorageBufferDynamic,
    AccelerationStructure,
}

#[derive(Copy, Clone, Hash, Debug)]
pub struct DescriptorBinding {
    pub binding: u32,
    pub descriptor_type: DescriptorType,
    pub count: u32,
}

pub trait DescriptorSetLayout {
    const LAYOUT_HASH: u64;
    fn create_layout(device: &Arc<AshDevice>) -> vk::DescriptorSetLayout;
}

pub struct DescriptorSet<T: DescriptorSetLayout> {
    data: T,
    descriptor_set: vk::DescriptorSet,
}
