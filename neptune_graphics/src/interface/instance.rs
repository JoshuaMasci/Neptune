use crate::interface::{Device, DeviceInfo, Surface};
use std::sync::Arc;

pub trait Instance {
    type DeviceImpl: Device;

    fn create_surface(&mut self) -> Option<Arc<Surface>>;

    /// Evaluates all available devices and assigns a score them, 0 means device is not usable, highest scoring device is initialized and returned.
    fn select_and_create_device(
        &mut self,
        surface: Option<Arc<Surface>>,
        score_function: impl Fn(&DeviceInfo) -> u32,
    ) -> Option<Self::DeviceImpl>;
}
