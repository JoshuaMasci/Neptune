use crate::device::DeviceSettings;
use crate::instance::AshInstance;
use crate::{Device, SurfaceHandle, VulkanError};
use ash::vk;
use log::error;
use std::sync::Arc;

#[derive(Clone)]
pub struct PhysicalDevice {
    pub(crate) instance: Arc<AshInstance>,
    pub(crate) index: usize,
    pub(crate) physical_device: vk::PhysicalDevice,

    pub(crate) properties: vk::PhysicalDeviceProperties,
    pub(crate) driver_properties: vk::PhysicalDeviceDriverProperties,
    pub(crate) queue_family_properties: Vec<vk::QueueFamilyProperties>,
}

impl PhysicalDevice {
    pub(crate) fn new(
        instance: Arc<AshInstance>,
        index: usize,
        physical_device: vk::PhysicalDevice,
    ) -> Self {
        let mut driver_properties = vk::PhysicalDeviceDriverProperties::default();
        let mut properties2 =
            vk::PhysicalDeviceProperties2::builder().push_next(&mut driver_properties);

        unsafe {
            instance
                .core
                .get_physical_device_properties2(physical_device, &mut properties2);
        };

        let queue_family_properties = unsafe {
            instance
                .core
                .get_physical_device_queue_family_properties(physical_device)
        };

        Self {
            instance,
            index,
            physical_device,
            properties: properties2.properties,
            driver_properties,
            queue_family_properties,
        }
    }

    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn get_properties(&self) -> &vk::PhysicalDeviceProperties {
        &self.properties
    }

    pub fn get_driver_properties(&self) -> &vk::PhysicalDeviceDriverProperties {
        &self.driver_properties
    }

    pub fn get_queue_family_properties(&self) -> &[vk::QueueFamilyProperties] {
        &self.queue_family_properties
    }

    pub fn get_surface_support(
        &self,
        queue_family_index: usize,
        surface_handle: SurfaceHandle,
    ) -> bool {
        if let Some(surface) = self.instance.surface_list.get(surface_handle.0) {
            unsafe {
                match self.instance.surface.get_physical_device_surface_support(
                    self.physical_device,
                    queue_family_index as u32,
                    surface,
                ) {
                    Ok(supported) => supported,
                    Err(err) => {
                        error!("vkGetPhysicalDeviceSurfaceSupportKHR failed: {}", err);
                        false
                    }
                }
            }
        } else {
            false
        }
    }

    pub fn create_device(&self, settings: &DeviceSettings) -> Result<Device, VulkanError> {
        Device::new(self.instance.clone(), self.physical_device, settings)
    }
}
