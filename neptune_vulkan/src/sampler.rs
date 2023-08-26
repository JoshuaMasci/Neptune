use crate::device::AshDevice;
use ash::vk;
use std::sync::Arc;

pub struct Sampler {
    device: Arc<AshDevice>,
    pub handle: vk::Sampler,
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe {
            self.device.core.destroy_sampler(self.handle, None);
        }
    }
}
