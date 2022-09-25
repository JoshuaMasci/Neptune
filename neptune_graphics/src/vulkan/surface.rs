use ash::vk;
use std::rc::Rc;

pub struct VulkanSurface {
    handle: vk::SurfaceKHR,
    surface_ext: Rc<ash::extensions::khr::Surface>,
}

impl VulkanSurface {
    pub(crate) fn new(
        handle: vk::SurfaceKHR,
        surface_ext: Rc<ash::extensions::khr::Surface>,
    ) -> Self {
        Self {
            handle,
            surface_ext,
        }
    }

    pub(crate) fn get_handle(&self) -> vk::SurfaceKHR {
        self.handle
    }
}

impl Drop for VulkanSurface {
    fn drop(&mut self) {
        unsafe {
            self.surface_ext.destroy_surface(self.handle, None);
        }
    }
}
