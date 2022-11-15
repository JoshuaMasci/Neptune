use crate::AshDevice;
use ash::vk;
use std::sync::Arc;

pub struct Sampler {
    device: Arc<AshDevice>,
    pub handle: vk::Sampler,
}

impl Sampler {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        create_info: &vk::SamplerCreateInfo,
    ) -> crate::Result<Self> {
        let handle = match unsafe { device.create_sampler(&create_info, None) } {
            Ok(handle) => handle,
            Err(e) => return Err(crate::Error::VkError(e)),
        };

        Ok(Self { device, handle })
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe { self.device.destroy_sampler(self.handle, None) }
        trace!("Drop Sampler");
    }
}
