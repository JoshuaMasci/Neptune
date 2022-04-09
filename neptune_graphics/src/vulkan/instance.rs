use crate::vulkan::debug_messenger::DebugMessenger;
use crate::vulkan::Device;
use ash::extensions::khr::Surface;
use ash::vk;
use std::ffi::CString;

pub struct Instance {
    entry: ash::Entry,
    surface_ext: Surface,
    instance: ash::Instance,
    surface: vk::SurfaceKHR,
    debug_messenger: Option<DebugMessenger>,
}

impl Instance {
    pub fn new(window: &winit::window::Window, app_name: &str, validation: bool) -> Self {
        let app_name = CString::new(app_name).unwrap();
        let app_version = vk::make_api_version(0, 0, 0, 0);

        let engine_name: CString = CString::new("Neptune Engine").unwrap();
        let engine_version = vk::make_api_version(0, 0, 0, 0);

        let entry = unsafe { ash::Entry::load() }.expect("Failed to create Vulkan Entry!");

        let mut layer_names = vec![];

        if validation {
            layer_names.push(CString::new("VK_LAYER_KHRONOS_validation").unwrap());
        }

        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let surface_extensions = ash_window::enumerate_required_extensions(window)
            .expect("Failed to get required surface extensions");

        let mut extension_names_raw = surface_extensions
            .iter()
            .map(|&ext| ext.as_ptr())
            .collect::<Vec<_>>();

        extension_names_raw.push(ash::extensions::ext::DebugUtils::name().as_ptr());
        extension_names_raw
            .push(ash::extensions::khr::GetPhysicalDeviceProperties2::name().as_ptr());

        let app_info = vk::ApplicationInfo::builder()
            .application_name(app_name.as_c_str())
            .application_version(app_version)
            .engine_name(engine_name.as_c_str())
            .engine_version(engine_version)
            .api_version(vk::API_VERSION_1_3);

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names_raw);

        let instance: ash::Instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("Failed to create vulkan instance")
        };

        let debug_messenger = if validation {
            Some(crate::vulkan::debug_messenger::DebugMessenger::new(
                &entry, &instance,
            ))
        } else {
            None
        };

        let surface_ext = ash::extensions::khr::Surface::new(&entry, &instance);
        let surface = unsafe {
            ash_window::create_surface(&entry, &instance, window, None)
                .expect("Failed to create vulkan surface")
        };

        Self {
            entry,
            surface_ext,
            instance,
            surface,
            debug_messenger,
        }
    }

    //TODO: help choose device!!!!
    pub fn create_device(&self, index: usize, frame_in_flight_count: u32) -> Device {
        let devices = unsafe { self.instance.enumerate_physical_devices() }
            .expect("Failed to get physical devices");
        let selected_device = devices[index];
        let selected_queue = 0; //TODO: this

        unsafe {
            if !self
                .surface_ext
                .get_physical_device_surface_support(selected_device, selected_queue, self.surface)
                .expect("Failed to check device support")
            {
                panic!("Selected device doesn't support the surface");
            }
        }

        Device::new(
            &self.instance,
            selected_device,
            selected_queue,
            frame_in_flight_count,
        )
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.surface_ext.destroy_surface(self.surface, None);
        }

        if let Some(debug_messenger) = self.debug_messenger.take() {
            drop(debug_messenger);
        }
        unsafe {
            self.instance.destroy_instance(None);
        }
        let _ = self.entry.clone();
    }
}
