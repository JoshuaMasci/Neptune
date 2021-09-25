use ash::*;

use winit;

use std::ffi::CStr;
use std::ffi::CString;

use crate::device::Device;

pub struct Instance {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub debug_utils_loader: ash::extensions::ext::DebugUtils,
    pub debug_call_back: vk::DebugUtilsMessengerEXT,
    pub surface_loader: ash::extensions::khr::Surface,
    pub surface: vk::SurfaceKHR,
    //pub device: Device,
}

pub struct AppVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl AppVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

pub struct AppInfo {
    pub name: String,
    pub version: AppVersion,
}

const SATURN_VERSION: u32 = vk::make_version(0, 0, 0);

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    use std::borrow::Cow;
    let callback_data = *p_callback_data;
    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    if message_severity != vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        println!("Vulkan {:?}: {}", message_severity, message,);
    }

    vk::FALSE
}

impl Instance {
    pub fn new(app: &AppInfo, window: &winit::window::Window) -> Self {
        let engine_name: CString = CString::new("Saturn Engine").unwrap();

        let entry = unsafe { Entry::new().unwrap() };

        let layer_names = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let surface_extensions = ash_window::enumerate_required_extensions(window).unwrap();
        let mut extension_names_raw = surface_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();
        extension_names_raw.push(ash::extensions::ext::DebugUtils::name().as_ptr());
        extension_names_raw
            .push(ash::extensions::khr::GetPhysicalDeviceProperties2::name().as_ptr());

        let temp_name = CString::new(app.name.as_str()).unwrap();
        let appinfo = vk::ApplicationInfo::builder()
            .application_name(temp_name.as_c_str())
            .application_version(SATURN_VERSION)
            .engine_name(engine_name.as_c_str())
            .engine_version(SATURN_VERSION)
            .api_version(vk::API_VERSION_1_2);

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names_raw);

        let ash_instance: ash::Instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("Instance creation error")
        };

        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(vulkan_debug_callback));

        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(&entry, &ash_instance);
        let debug_call_back = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap()
        };

        let surface_loader = ash::extensions::khr::Surface::new(&entry, &ash_instance);
        let surface =
            unsafe { ash_window::create_surface(&entry, &ash_instance, window, None).unwrap() };

        Self {
            entry,
            instance: ash_instance,

            debug_utils_loader,
            debug_call_back,
            surface_loader,
            surface,
        }
    }

    pub fn create_device(&mut self, index: usize) -> Device {
        let pdevices = unsafe {
            self.instance
                .enumerate_physical_devices()
                .expect("Failed to enumerate devices")
        };

        unsafe {
            if self
                .surface_loader
                .get_physical_device_surface_support(pdevices[index], 0, self.surface)
                .expect("Failed to check device support")
                == false
            {
                panic!("Selected Device doesn't support the surface");
            }
        }

        Device::new(
            self.instance.clone(),
            pdevices[index],
            0,
            self.surface,
            &self.surface_loader,
        )
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
            self.instance.destroy_instance(None);
        }
    }
}
