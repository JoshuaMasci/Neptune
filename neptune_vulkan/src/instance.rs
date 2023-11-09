use crate::debug_utils::DebugUtils;
use crate::interface::PhysicalDevice;
use crate::{SurfaceHandle, SurfaceKey, VulkanError};
use ash::prelude::VkResult;
use ash::vk;
use log::trace;
use slotmap::SlotMap;
use std::ffi::CString;
use std::sync::{Arc, Mutex};

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

pub struct SurfaceList(Mutex<SlotMap<SurfaceKey, vk::SurfaceKHR>>);

impl SurfaceList {
    pub fn new() -> Self {
        Self(Mutex::new(SlotMap::with_key()))
    }

    pub fn insert(&self, surface: vk::SurfaceKHR) -> SurfaceKey {
        self.0.lock().unwrap().insert(surface)
    }

    pub fn remove(&self, surface_key: SurfaceKey) -> Option<vk::SurfaceKHR> {
        self.0.lock().unwrap().remove(surface_key)
    }

    pub fn get(&self, surface_key: SurfaceKey) -> Option<vk::SurfaceKHR> {
        self.0.lock().unwrap().get(surface_key).cloned()
    }
}

pub struct AshInstance {
    pub entry: ash::Entry,
    pub core: ash::Instance,
    pub surface: ash::extensions::khr::Surface,
    pub debug_utils: Option<DebugUtils>,

    pub(crate) surface_list: SurfaceList,
}

impl AshInstance {
    pub fn new(
        engine_info: &AppInfo,
        app_info: &AppInfo,
        enable_debug: bool,
        display_handle: Option<raw_window_handle::RawDisplayHandle>,
    ) -> VkResult<Self> {
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
            Err(_) => return Err(vk::Result::ERROR_INITIALIZATION_FAILED),
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
            extension_names_raw
                .push(ash::extensions::khr::GetPhysicalDeviceProperties2::name().as_ptr());
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
            surface_list: SurfaceList::new(),
        })
    }

    pub fn crate_surface(
        &self,
        display_handle: raw_window_handle::RawDisplayHandle,
        window_handle: raw_window_handle::RawWindowHandle,
    ) -> VkResult<SurfaceKey> {
        match unsafe {
            ash_window::create_surface(&self.entry, &self.core, display_handle, window_handle, None)
        } {
            Ok(surface) => Ok(self.surface_list.insert(surface)),
            Err(err) => Err(err),
        }
    }

    pub fn destroy_surface(&self, surface_key: SurfaceKey) {
        if let Some(surface) = self.surface_list.remove(surface_key) {
            unsafe {
                self.surface.destroy_surface(surface, None);
            }
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

pub struct Instance {
    pub(crate) instance: Arc<AshInstance>,
    pub(crate) physical_devices: Vec<PhysicalDevice>,
}

impl Instance {
    pub fn new(
        engine_info: &AppInfo,
        app_info: &AppInfo,
        display_handle: Option<raw_window_handle::RawDisplayHandle>,
    ) -> Result<Self, VulkanError> {
        let instance =
            AshInstance::new(engine_info, app_info, true, display_handle).map(Arc::new)?;

        let physical_devices = unsafe { instance.core.enumerate_physical_devices() }
            .expect("Failed to enumerate physical devices")
            .iter()
            .enumerate()
            .map(|(index, &physical_device)| {
                PhysicalDevice::new(instance.clone(), index, physical_device)
            })
            .collect();

        Ok(Self {
            instance,
            physical_devices,
        })
    }

    pub fn create_surface(
        &mut self,
        raw_display_handle: raw_window_handle::RawDisplayHandle,
        raw_window_handle: raw_window_handle::RawWindowHandle,
    ) -> Result<SurfaceHandle, VulkanError> {
        let surface_key = self
            .instance
            .crate_surface(raw_display_handle, raw_window_handle)?;
        Ok(SurfaceHandle(surface_key))
    }

    pub fn destroy_surface(&mut self, surface_handle: SurfaceHandle) {
        self.instance.destroy_surface(surface_handle.0)
    }

    pub fn get_physical_device(&self, index: usize) -> Option<PhysicalDevice> {
        self.physical_devices.get(index).cloned()
    }

    pub fn select_physical_device(
        &self,
        score_function: impl Fn(&PhysicalDevice) -> Option<usize>,
    ) -> Option<PhysicalDevice> {
        let highest_scored_device_index: Option<usize> = self
            .physical_devices
            .iter()
            .map(|physical_device| (physical_device.get_index(), score_function(physical_device)))
            .filter(|(_index, score)| score.is_some())
            .max_by_key(|(_index, score)| score.unwrap())
            .map(|(index, _score)| index);
        highest_scored_device_index
            .and_then(|device_index| self.physical_devices.get(device_index))
            .cloned()
    }
}
