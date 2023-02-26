use crate::interfaces::Device;
use crate::traits::InstanceTrait;
use crate::vulkan::debug_utils::DebugUtils;
use crate::vulkan::AshSurfaceHandle;
use crate::{AppInfo, DeviceType, DeviceVendor, PhysicalDeviceInfo, SurfaceHandle};
use ash::prelude::VkResult;
use ash::vk;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use slotmap::{KeyData, SlotMap};
use std::ffi::{c_char, CStr, CString};
use std::sync::{Arc, Mutex};

fn c_char_to_string(char_slice: &[c_char]) -> String {
    unsafe {
        CStr::from_ptr(char_slice.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}

fn get_device_type(device_type: vk::PhysicalDeviceType) -> DeviceType {
    match device_type {
        vk::PhysicalDeviceType::DISCRETE_GPU => DeviceType::Discrete,
        vk::PhysicalDeviceType::INTEGRATED_GPU => DeviceType::Integrated,
        _ => DeviceType::Unknown,
    }
}

fn get_device_vendor(vendor_id: u32) -> DeviceVendor {
    //List from here: https://www.reddit.com/r/vulkan/comments/4ta9nj/is_there_a_comprehensive_list_of_the_names_and/
    //TODO: find a more complete list?
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

pub struct AshPhysicalDevice {
    pub(crate) handle: vk::PhysicalDevice,
    pub(crate) info: PhysicalDeviceInfo,
    pub(crate) graphics_queue_family_index: u32,
    pub(crate) transfer_queue_family_index: Option<u32>,
    pub(crate) compute_queue_family_index: Option<u32>,
}

impl AshPhysicalDevice {
    fn new(instance: &ash::Instance, handle: vk::PhysicalDevice) -> Self {
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(handle) };

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

        let properties = unsafe { instance.get_physical_device_properties(handle) };

        let info = PhysicalDeviceInfo {
            name: c_char_to_string(&properties.device_name),
            device_type: get_device_type(properties.device_type),
            vendor: get_device_vendor(properties.vendor_id),
            driver: format!("{:x}", properties.driver_version),
        };

        Self {
            handle,
            info,
            graphics_queue_family_index,
            transfer_queue_family_index: None,
            compute_queue_family_index: None,
        }
    }

    fn get_surface_support(
        &self,
        surface_ext: &Arc<ash::extensions::khr::Surface>,
        surface: Option<vk::SurfaceKHR>,
    ) -> bool {
        if let Some(surface) = surface {
            unsafe {
                surface_ext.get_physical_device_surface_support(
                    self.handle,
                    self.graphics_queue_family_index,
                    surface,
                )
            }
            .unwrap_or(false)
        } else {
            true
        }
    }
}

pub(crate) struct AshSurface {
    handle: vk::SurfaceKHR,
    surface_ext: Arc<ash::extensions::khr::Surface>,
}
impl AshSurface {
    pub(crate) fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        surface_ext: Arc<ash::extensions::khr::Surface>,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> ash::prelude::VkResult<Self> {
        Ok(Self {
            handle: unsafe {
                ash_window::create_surface(entry, instance, display_handle, window_handle, None)
            }?,
            surface_ext,
        })
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
        //TODO: can both be initialized at the same time?
        extension_names_raw.push(ash::extensions::khr::XlibSurface::name().as_ptr());
        extension_names_raw.push(ash::extensions::khr::WaylandSurface::name().as_ptr());
    }
}

pub struct AshInstance {
    pub(crate) entry: ash::Entry,
    pub(crate) surface_ext: Arc<ash::extensions::khr::Surface>,
    pub(crate) debug_utils: Option<Arc<DebugUtils>>,
    pub(crate) handle: ash::Instance,
}

impl Drop for AshInstance {
    fn drop(&mut self) {
        //Drop the debug_utils before instance
        drop(self.debug_utils.take());
        unsafe {
            self.handle.destroy_instance(None);
        }
        let _ = self.entry.clone();
    }
}

pub struct Instance {
    instance: Arc<AshInstance>,
    physical_devices: Vec<AshPhysicalDevice>,
    surfaces: Arc<Mutex<SlotMap<AshSurfaceHandle, AshSurface>>>,
}

impl Instance {
    pub fn new(engine_info: &AppInfo, app_info: &AppInfo) -> ash::prelude::VkResult<Self> {
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
            Err(e) => return Err(ash::vk::Result::ERROR_INITIALIZATION_FAILED),
        };

        let mut layer_names_raw = Vec::new();

        let mut extension_names_raw = vec![ash::extensions::khr::Surface::name().as_ptr()];
        get_surface_extensions(&mut extension_names_raw);

        //TODO: enable or disable
        let enable_debug = true;

        //Name must persist until create_instance is called
        let validation_layer_name = CString::new("VK_LAYER_KHRONOS_validation").unwrap();

        if enable_debug {
            layer_names_raw.push(validation_layer_name.as_ptr());
            extension_names_raw.push(ash::extensions::ext::DebugUtils::name().as_ptr());
        }

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

        let instance: ash::Instance = unsafe { entry.create_instance(&create_info, None)? };

        let surface_ext = Arc::new(ash::extensions::khr::Surface::new(&entry, &instance));

        let debug_utils = if enable_debug {
            Some(DebugUtils::new(&entry, &instance).map(Arc::new)?)
        } else {
            None
        };

        let ash_instance = Arc::new(AshInstance {
            entry,
            surface_ext,
            debug_utils,
            handle: instance,
        });

        let physical_devices = unsafe { ash_instance.handle.enumerate_physical_devices()? }
            .iter()
            .map(|handle| AshPhysicalDevice::new(&ash_instance.handle, handle.clone()))
            .collect();

        let surfaces = Arc::new(Mutex::new(SlotMap::with_key()));

        Ok(Self {
            instance: ash_instance,
            physical_devices,
            surfaces,
        })
    }
}

impl Drop for Instance {
    fn drop(&mut self) {}
}

impl InstanceTrait for Instance {
    fn create_surface(
        &self,
        name: &str,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> crate::Result<SurfaceHandle> {
        let surface = match AshSurface::new(
            &self.instance.entry,
            &self.instance.handle,
            self.instance.surface_ext.clone(),
            display_handle,
            window_handle,
        ) {
            Ok(surface) => surface,
            Err(_e) => return Err(crate::Error::TempError),
        };

        Ok(self.surfaces.lock().unwrap().insert(surface).0.as_ffi())
    }

    fn destroy_surface(&self, handle: SurfaceHandle) {
        drop(
            self.surfaces
                .lock()
                .unwrap()
                .remove(AshSurfaceHandle::from(KeyData::from_ffi(handle))),
        );
    }

    fn get_supported_devices(
        &self,
        surface: Option<SurfaceHandle>,
    ) -> Vec<(usize, PhysicalDeviceInfo)> {
        let surface: Option<vk::SurfaceKHR> = surface.and_then(|handle| {
            self.surfaces
                .lock()
                .unwrap()
                .get(AshSurfaceHandle::from(KeyData::from_ffi(handle)))
                .map(|surface| surface.handle)
        });

        self.physical_devices
            .iter()
            .enumerate()
            .filter(|(_index, physical_device)| {
                physical_device.get_surface_support(&self.instance.surface_ext, surface)
            })
            .map(|(index, physical_device)| (index, physical_device.info.clone()))
            .collect()
    }

    fn create_device(&self, index: usize, frames_in_flight_count: usize) -> crate::Result<Device> {
        todo!()
    }
}
