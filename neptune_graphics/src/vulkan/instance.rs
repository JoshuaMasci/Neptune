use crate::device::DeviceInfo;
use crate::vulkan::device::VulkanDevice;
use crate::vulkan::surface::VulkanSurface;
use crate::{DeviceType, DeviceVendor, InstanceTrait};
use ash::vk;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::rc::Rc;

impl DeviceType {
    fn from_vk(device_type: vk::PhysicalDeviceType) -> Self {
        match device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => Self::Discrete,
            vk::PhysicalDeviceType::INTEGRATED_GPU => Self::Integrated,
            _ => Self::Unknown,
        }
    }
}

impl DeviceVendor {
    fn from_vk(vendor_id: u32) -> Self {
        match vendor_id {
            0x1002 => DeviceVendor::Amd,
            0x10DE => DeviceVendor::Nvidia,
            0x8086 => DeviceVendor::Intel,
            0x1010 => DeviceVendor::ImgTec,
            0x13B5 => DeviceVendor::Arm,
            0x5132 => DeviceVendor::Qualcomm,
            x => DeviceVendor::Unknown(x),
        }
    }
}

impl DeviceInfo {
    fn from_vk(physical_device_properties: vk::PhysicalDeviceProperties) -> Self {
        Self {
            name: String::from(
                unsafe { CStr::from_ptr(physical_device_properties.device_name.as_ptr()) }
                    .to_str()
                    .expect("Failed to convert CStr to string"),
            ),
            vendor: DeviceVendor::from_vk(physical_device_properties.vendor_id),
            device_type: DeviceType::from_vk(physical_device_properties.device_type),
        }
    }
}

pub(crate) struct PhysicalDevice {
    pub(crate) handle: vk::PhysicalDevice,
    pub(crate) device_info: DeviceInfo,

    //TODO: more complicated queue layout
    pub(crate) graphics_queue_family_index: u32,
}

impl PhysicalDevice {
    fn new(instance: &ash::Instance, physical_device: vk::PhysicalDevice) -> Self {
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        //TODO: for the moment, only choose a queue family that supports all operations
        // This will work for most desktop GPU's, as they will have this type of queue family
        // However I would still like to make a more robust queue family selection system for the other GPU's
        let graphics_queue_family_index = queue_family_properties
            .iter()
            .enumerate()
            .find(|(_index, &queue_family)| {
                queue_family.queue_flags.contains(
                    vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER,
                )
            })
            .unwrap()
            .0 as u32;

        Self {
            handle: physical_device,
            device_info: DeviceInfo::from_vk(unsafe {
                instance.get_physical_device_properties(physical_device)
            }),
            graphics_queue_family_index,
        }
    }

    fn get_surface_support(
        &self,
        surface_ext: &Rc<ash::extensions::khr::Surface>,
        surface: vk::SurfaceKHR,
    ) -> bool {
        unsafe {
            surface_ext.get_physical_device_surface_support(
                self.handle,
                self.graphics_queue_family_index,
                surface,
            )
        }
        .unwrap_or(false)
    }
}

fn get_surface_extensions(extension_names_raw: &mut Vec<*const c_char>) {
    #[cfg(target_os = "windows")]
    {
        extension_names_raw.push(ash::extensions::khr::Win32Surface::name().as_ptr());
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        extension_names_raw.push(ash::extensions::khr::XlibSurface::name().as_ptr());
        extension_names_raw.push(ash::extensions::khr::WaylandSurface::name().as_ptr());
    }
}

pub struct VulkanInstance {
    entry: ash::Entry,
    surface_ext: Rc<ash::extensions::khr::Surface>,
    instance: ash::Instance,

    physical_devices: Vec<PhysicalDevice>,
}

impl VulkanInstance {
    pub fn new(app_name: &str) -> Self {
        let app_name = CString::new(app_name).unwrap();
        let app_version = vk::make_api_version(0, 0, 0, 0);

        let engine_name: CString = CString::new("Neptune Vulkan Backend").unwrap();
        let engine_version = vk::make_api_version(0, 0, 0, 0);

        let entry = unsafe { ash::Entry::load() }.expect("Failed to create Vulkan Entry!");

        let mut layer_names_raw = Vec::new();

        //TODO: enable or disable
        let validation_layer_name = CString::new("VK_LAYER_KHRONOS_validation").unwrap();
        layer_names_raw.push(validation_layer_name.as_ptr());

        let mut extension_names_raw = vec![ash::extensions::khr::Surface::name().as_ptr()];
        get_surface_extensions(&mut extension_names_raw);

        let app_info = vk::ApplicationInfo::builder()
            .application_name(app_name.as_c_str())
            .application_version(app_version)
            .engine_name(engine_name.as_c_str())
            .engine_version(engine_version)
            .api_version(vk::API_VERSION_1_3);

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layer_names_raw)
            .enabled_extension_names(&extension_names_raw);

        let instance: ash::Instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("Failed to create vulkan instance")
        };

        let surface_ext = Rc::new(ash::extensions::khr::Surface::new(&entry, &instance));

        let physical_devices = unsafe { instance.enumerate_physical_devices() }
            .expect("Failed to enumerate physical devices")
            .iter()
            .map(|&physical_device| PhysicalDevice::new(&instance, physical_device))
            .collect();

        Self {
            entry,
            surface_ext,
            instance,
            physical_devices,
        }
    }
}

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }

        let _ = self.entry.clone();
    }
}

impl InstanceTrait for VulkanInstance {
    type DeviceImpl = VulkanDevice;
    type SurfaceImpl = VulkanSurface;

    fn create_surface<
        T: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    >(
        &mut self,
        window: &T,
    ) -> Option<Self::SurfaceImpl> {
        unsafe {
            ash_window::create_surface(
                &self.entry,
                &self.instance,
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )
            .ok()
            .map(|handle| VulkanSurface::new(handle, self.surface_ext.clone()))
        }
    }

    fn select_and_create_device(
        &mut self,
        surface: Option<&Self::SurfaceImpl>,
        score_function: impl Fn(&DeviceInfo) -> u32,
    ) -> Option<Self::DeviceImpl> {
        let max_score = self
            .physical_devices
            .iter()
            .enumerate()
            .map(|(index, physical_device)| {
                (
                    index,
                    if let Some(surface) = surface {
                        if physical_device
                            .get_surface_support(&self.surface_ext, surface.get_handle())
                        {
                            score_function(&physical_device.device_info)
                        } else {
                            0
                        }
                    } else {
                        score_function(&physical_device.device_info)
                    },
                )
            })
            .max_by_key(|index_score| index_score.1);
        max_score
            .map(|(index, _score)| VulkanDevice::new(&self.instance, &self.physical_devices[index]))
    }
}
