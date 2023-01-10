use crate::{Device, Error, GpuResourcePool, SurfaceHandle};
use crate::{DeviceInfo, GpuResource};

use crate::debug_utils::DebugUtils;
use crate::surface::{AshSurface, Surface};
use ash::vk;
use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::Arc;

#[derive(Clone)]
pub struct PhysicalDevice {
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
            device_info: DeviceInfo::new(unsafe {
                instance.get_physical_device_properties(physical_device)
            }),
            graphics_queue_family_index,
        }
    }

    fn get_surface_support(
        &self,
        surface_ext: &Arc<ash::extensions::khr::Surface>,
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

pub struct Instance {
    instance: Arc<AshInstance>,
}

impl Instance {
    pub fn new(app_name: &str) -> crate::Result<Self> {
        AshInstance::new(app_name)
            .map(Arc::new)
            .map(|instance| Self { instance })
    }

    pub fn create_surface<
        T: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    >(
        &mut self,
        window: &T,
    ) -> crate::Result<Surface> {
        let ash_surface = Arc::new(AshSurface::new(
            &self.instance.entry,
            &self.instance.instance,
            self.instance.surface_ext.clone(),
            window,
        )?);
        let handle = self.instance.surfaces.lock().unwrap().insert(ash_surface);
        Ok(Surface(GpuResource::new(
            handle,
            self.instance.surfaces.clone(),
        )))
    }

    pub fn select_and_create_device(
        &self,
        surface: Option<&Surface>,
        score_function: impl Fn(&DeviceInfo) -> u32,
    ) -> crate::Result<Device> {
        let surface_handle: Option<vk::SurfaceKHR> = surface.map(|surface| {
            surface
                .0
                .pool
                .lock()
                .unwrap()
                .get(surface.0.handle)
                .unwrap()
                .get_handle()
        });

        let max_score = self
            .instance
            .physical_devices
            .iter()
            .enumerate()
            .map(|(index, physical_device)| {
                (
                    index,
                    if let Some(surface) = surface_handle {
                        if physical_device.get_surface_support(&self.instance.surface_ext, surface)
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

        match max_score {
            Some((index, _score)) => Device::new(self.instance.clone(), index),
            None => Err(Error::StringError(String::from(
                "Unable to find valid device",
            ))),
        }
    }
}

pub struct AshInstance {
    pub(crate) entry: ash::Entry,
    pub(crate) surface_ext: Arc<ash::extensions::khr::Surface>,
    pub(crate) debug_utils: Option<Arc<DebugUtils>>,
    pub(crate) instance: ash::Instance,
    pub(crate) physical_devices: Vec<PhysicalDevice>,
    pub(crate) surfaces: GpuResourcePool<SurfaceHandle, Arc<AshSurface>>,
}

impl AshInstance {
    pub fn new(app_name: &str) -> crate::Result<Self> {
        let app_name = CString::new(app_name).unwrap();
        let app_version = vk::make_api_version(0, 0, 0, 0);

        let engine_name: CString = CString::new("Neptune Vulkan Backend").unwrap();
        let engine_version = vk::make_api_version(0, 0, 0, 0);

        let entry = match unsafe { ash::Entry::load() } {
            Ok(entry) => entry,
            Err(e) => {
                return Err(Error::StringError(format!(
                    "Failed to create vulkan entry: {}",
                    e
                )))
            }
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

        let instance: ash::Instance = match unsafe { entry.create_instance(&create_info, None) } {
            Ok(instance) => instance,
            Err(e) => return Err(Error::VkError(e)),
        };

        let debug_utils = if enable_debug {
            Some(DebugUtils::new(&entry, &instance).map(Arc::new)?)
        } else {
            None
        };

        let surface_ext = Arc::new(ash::extensions::khr::Surface::new(&entry, &instance));

        let physical_devices = match unsafe { instance.enumerate_physical_devices() } {
            Ok(physical_devices) => physical_devices,
            Err(e) => return Err(Error::VkError(e)),
        }
        .iter()
        .map(|&physical_device| PhysicalDevice::new(&instance, physical_device))
        .collect();

        let surfaces = GpuResourcePool::new();

        Ok(Self {
            entry,
            surface_ext,
            debug_utils,
            instance,
            physical_devices,
            surfaces,
        })
    }
}

impl Drop for AshInstance {
    fn drop(&mut self) {
        //Drop the debug_utils before instance
        drop(self.debug_utils.take());

        unsafe {
            self.instance.destroy_instance(None);
        }
        let _ = self.entry.clone();
        trace!("Drop Instance");
    }
}
