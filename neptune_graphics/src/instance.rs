use crate::device::DeviceInfo;
use crate::surface::Surface;
use crate::DeviceTrait;

pub trait InstanceTrait {
    type DeviceImpl: DeviceTrait;

    fn create_surface(&mut self) -> Option<Surface>;

    /// Evaluates all available devices and assigns a score them, 0 means device is not usable, highest scoring device is initialized and returned.
    fn select_and_create_device(
        &mut self,
        surface: Option<&Surface>,
        score_function: impl Fn(&DeviceInfo) -> u32,
    ) -> Option<Self::DeviceImpl>;
}
