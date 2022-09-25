use crate::device::DeviceInfo;
use crate::DeviceTrait;

pub enum Backend {
    PlatformPreferred,
    Vulkan,
    Dx12,
}

pub trait InstanceTrait {
    type DeviceImpl: DeviceTrait;
    type SurfaceImpl; //TODO: Trait?

    fn create_surface<
        T: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    >(
        &mut self,
        window: &T,
    ) -> Option<Self::SurfaceImpl>;

    /// Evaluates all available devices and assigns a score them, 0 means device is not usable, highest scoring device is initialized and returned.
    fn select_and_create_device(
        &mut self,
        surface: Option<&Self::SurfaceImpl>,
        score_function: impl Fn(&DeviceInfo) -> u32,
    ) -> Option<Self::DeviceImpl>;
}
