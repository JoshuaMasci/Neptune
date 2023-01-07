use crate::GpuResource;
use ash::vk;
use std::sync::Arc;

#[repr(transparent)]
pub struct Surface(pub(crate) GpuResource<crate::SurfaceHandle, Arc<AshSurface>>);

pub(crate) struct AshSurface {
    handle: vk::SurfaceKHR,
    surface_ext: Arc<ash::extensions::khr::Surface>,
}
impl AshSurface {
    pub(crate) fn new<
        T: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    >(
        entry: &ash::Entry,
        instance: &ash::Instance,
        surface_ext: Arc<ash::extensions::khr::Surface>,
        window: &T,
    ) -> crate::Result<Self> {
        match unsafe {
            ash_window::create_surface(
                entry,
                instance,
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )
        } {
            Ok(handle) => crate::Result::Ok(Self {
                handle,
                surface_ext,
            }),
            Err(e) => crate::Result::Err(crate::Error::VkError(e)),
        }
    }

    pub fn get_handle(&self) -> vk::SurfaceKHR {
        self.handle
    }
}
impl Drop for AshSurface {
    fn drop(&mut self) {
        unsafe {
            self.surface_ext.destroy_surface(self.handle, None);
        }
    }
}
