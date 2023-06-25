use crate::debug_utils::DebugUtils;
use ash::vk;
use log::trace;
use std::ffi::CString;

#[derive(Debug)]
pub struct AppInfo<'a> {
    pub(crate) name: &'a str,
    pub(crate) variant_version: u32,
    pub(crate) major_version: u32,
    pub(crate) minor_version: u32,
    pub(crate) patch_version: u32,
}

impl<'a> AppInfo<'a> {
    pub fn new(name: &'a str, version: [u32; 4]) -> Self {
        Self {
            name,
            variant_version: version[0],
            major_version: version[1],
            minor_version: version[2],
            patch_version: version[3],
        }
    }
}

pub struct AshInstance {
    pub entry: ash::Entry,
    pub core: ash::Instance,
    pub surface: ash::extensions::khr::Surface,
    pub debug_utils: Option<DebugUtils>,
}

impl AshInstance {
    pub fn new(
        engine_info: &AppInfo,
        app_info: &AppInfo,
        enable_debug: bool,
        display_handle: Option<raw_window_handle::RawDisplayHandle>,
    ) -> ash::prelude::VkResult<Self> {
        trace!(
            "Creating Vulkan Instance Engine: {:?}, App: {:?}",
            engine_info,
            app_info
        );

        let engine_name: CString = CString::new(engine_info.name).unwrap();
        let engine_version = vk::make_api_version(
            engine_info.variant_version,
            engine_info.major_version,
            engine_info.minor_version,
            engine_info.patch_version,
        );

        let app_name: CString = CString::new(app_info.name).unwrap();
        let app_version = vk::make_api_version(
            app_info.variant_version,
            app_info.major_version,
            app_info.minor_version,
            app_info.patch_version,
        );

        let entry = match unsafe { ash::Entry::load() } {
            Ok(entry) => entry,
            Err(_) => return Err(ash::vk::Result::ERROR_INITIALIZATION_FAILED),
        };

        let mut layer_names_raw = Vec::new();
        let mut extension_names_raw = vec![ash::extensions::khr::Surface::name().as_ptr()];

        if let Some(display_handle) = display_handle {
            let mut required_window_extensions =
                ash_window::enumerate_required_extensions(display_handle)?.to_vec();
            extension_names_raw.append(&mut required_window_extensions);
        }

        //Name must persist until create_instance is called
        let validation_layer_name = CString::new("VK_LAYER_KHRONOS_validation").unwrap();

        if enable_debug {
            layer_names_raw.push(validation_layer_name.as_ptr());
            extension_names_raw.push(ash::extensions::ext::DebugUtils::name().as_ptr());
        }

        let app_info = vk::ApplicationInfo::builder()
            .api_version(vk::API_VERSION_1_3)
            .application_name(app_name.as_c_str())
            .application_version(app_version)
            .engine_name(engine_name.as_c_str())
            .engine_version(engine_version);

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layer_names_raw)
            .enabled_extension_names(&extension_names_raw);

        let instance: ash::Instance = unsafe { entry.create_instance(&create_info, None)? };

        let surface = ash::extensions::khr::Surface::new(&entry, &instance);

        let debug_utils = if enable_debug {
            Some(DebugUtils::new(&entry, &instance)?)
        } else {
            None
        };

        Ok(Self {
            entry,
            core: instance,
            surface,
            debug_utils,
        })
    }

    pub fn crate_surface(
        &self,
        display_handle: raw_window_handle::RawDisplayHandle,
        window_handle: raw_window_handle::RawWindowHandle,
    ) -> ash::prelude::VkResult<vk::SurfaceKHR> {
        unsafe {
            ash_window::create_surface(&self.entry, &self.core, display_handle, window_handle, None)
        }
    }
}

impl Drop for AshInstance {
    fn drop(&mut self) {
        drop(self.debug_utils.take());
        unsafe {
            self.core.destroy_instance(None);
        }
    }
}
