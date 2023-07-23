use crate::vulkan::device::AshDevice;
use ash::vk;
use std::sync::Arc;

pub(crate) struct AshSampler {
    device: Arc<AshDevice>,
    pub(crate) handle: vk::Sampler,
}

impl AshSampler {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        sampler_create_info: &vk::SamplerCreateInfo,
    ) -> ash::prelude::VkResult<Self> {
        unsafe { device.core.create_sampler(sampler_create_info, None) }
            .map(|handle| Self { device, handle })
    }
}

impl Drop for AshSampler {
    fn drop(&mut self) {
        unsafe { self.device.core.destroy_sampler(self.handle, None) };
    }
}
