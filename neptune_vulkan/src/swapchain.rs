use crate::AshDevice;
use ash::vk;
use std::sync::Arc;

pub struct Swapchain {
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    device: Arc<AshDevice>,
    surface_ext: Arc<ash::extensions::khr::Surface>,
    swapchain_ext: Arc<ash::extensions::khr::Swapchain>,
}

impl Swapchain {
    pub(crate) fn new(
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        device: Arc<AshDevice>,
        surface_ext: Arc<ash::extensions::khr::Surface>,
        swapchain_ext: Arc<ash::extensions::khr::Swapchain>,
    ) -> crate::Result<Self> {
        Ok(Self {
            physical_device,
            surface,
            device,
            surface_ext,
            swapchain_ext,
        })
    }
}
