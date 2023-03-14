use crate::interfaces::Device;
use crate::traits::InstanceTrait;
use crate::vulkan::debug_utils::DebugUtils;
use crate::vulkan::AshSurfaceHandle;
use crate::{
    AppInfo, DeviceCreateInfo, DeviceType, DeviceVendor, PhysicalDeviceExtensions,
    PhysicalDeviceFeatures, PhysicalDeviceInfo, PhysicalDeviceMemory, SurfaceHandle,
};

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

#[derive(Clone)]
pub(crate) struct AshPhysicalDeviceQueues {
    pub(crate) primary_queue_family_index: u32,
    pub(crate) compute_queue_family_index: Option<u32>,
    pub(crate) transfer_queue_family_index: Option<u32>,
}

pub(crate) struct AshPhysicalDevice {
    pub(crate) handle: vk::PhysicalDevice,
    pub(crate) info: PhysicalDeviceInfo,
    pub(crate) queues: AshPhysicalDeviceQueues,
    pub(crate) extensions: PhysicalDeviceExtensions,
}

fn find_queue(
    queue_family_properties: &[vk::QueueFamilyProperties],
    contains_flags: vk::QueueFlags,
    exclude_flags: vk::QueueFlags,
) -> Option<u32> {
    queue_family_properties
        .iter()
        .enumerate()
        .find(|(_index, &queue_family)| {
            queue_family.queue_flags.contains(contains_flags)
                && !queue_family.queue_flags.intersects(exclude_flags)
        })
        .map(|(index, _queue_family)| index as u32)
}

fn supports_extension(
    supported_extensions: &[vk::ExtensionProperties],
    extension_name: &CStr,
) -> bool {
    supported_extensions.iter().any(|supported_extension| {
        let supported_extension_name =
            unsafe { CStr::from_ptr(supported_extension.extension_name.as_ptr()) };
        supported_extension_name == extension_name
    })
}

impl AshPhysicalDevice {
    fn new(instance: &ash::Instance, handle: vk::PhysicalDevice) -> Self {
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(handle) };

        //TODO: for the moment, only choose a queue family that supports all operations
        // This will work for most desktop GPU's, as they will have this type of queue family
        // However I would still like to make a more robust queue family selection system for the other GPU's
        let primary_queue_family_index = find_queue(
            &queue_family_properties,
            vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER,
            vk::QueueFlags::empty(),
        )
        .expect("Failed to find primary queue family, TODO: remove this expect statement");

        let compute_queue_family_index = find_queue(
            &queue_family_properties,
            vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER,
            vk::QueueFlags::GRAPHICS,
        );

        let transfer_queue_family_index = find_queue(
            &queue_family_properties,
            vk::QueueFlags::TRANSFER,
            vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE,
        );

        let properties = unsafe { instance.get_physical_device_properties(handle) };

        let supported_extensions: Vec<vk::ExtensionProperties> = unsafe {
            instance
                .enumerate_device_extension_properties(handle)
                .unwrap()
        };

        let extensions = PhysicalDeviceExtensions {
            dynamic_rendering: supports_extension(
                &supported_extensions,
                ash::extensions::khr::DynamicRendering::name(),
            ),
            mesh_shading: supports_extension(
                &supported_extensions,
                ash::extensions::ext::MeshShader::name(),
            ),
            ray_tracing: supports_extension(
                &supported_extensions,
                ash::extensions::khr::AccelerationStructure::name(),
            ) && supports_extension(
                &supported_extensions,
                ash::extensions::khr::RayTracingPipeline::name(),
            ) && supports_extension(
                &supported_extensions,
                ash::extensions::khr::DeferredHostOperations::name(),
            ),
        };

        let memory_properties = unsafe { instance.get_physical_device_memory_properties(handle) };
        let heap_slice = &memory_properties.memory_heaps[0..(memory_properties.memory_heap_count
            as usize)
            .min(memory_properties.memory_heaps.len())];
        let device_local_bytes: usize = heap_slice
            .iter()
            .map(|memory_heap| {
                if memory_heap
                    .flags
                    .contains(vk::MemoryHeapFlags::DEVICE_LOCAL)
                {
                    memory_heap.size as usize
                } else {
                    0
                }
            })
            .sum();

        let info = PhysicalDeviceInfo {
            name: c_char_to_string(&properties.device_name),
            device_type: get_device_type(properties.device_type),
            vendor: get_device_vendor(properties.vendor_id),
            driver: format!("{:x}", properties.driver_version),
            memory: PhysicalDeviceMemory { device_local_bytes },
            features: PhysicalDeviceFeatures {
                async_compute: compute_queue_family_index.is_some(),
                async_transfer: transfer_queue_family_index.is_some(),
            },
            extensions: extensions.clone(),
        };

        Self {
            handle,
            info,
            queues: AshPhysicalDeviceQueues {
                primary_queue_family_index,
                compute_queue_family_index,
                transfer_queue_family_index,
            },
            extensions,
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
                    self.queues.primary_queue_family_index,
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
    pub(crate) instance: Arc<AshInstance>,
    pub(crate) physical_devices: Vec<AshPhysicalDevice>,
    pub(crate) surfaces: Arc<Mutex<SlotMap<AshSurfaceHandle, AshSurface>>>,
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

        let _ = name;
        //TODO: Surface name cannot be set using debug without a device handle
        // if let Some(debug_utils) = &self.instance.debug_utils {
        //     let _ = debug_utils.set_object_name(vk::Device::null(), surface.handle, name);
        // }

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

    fn create_device(&self, index: usize, create_info: &DeviceCreateInfo) -> crate::Result<Device> {
        Ok(crate::Device {
            device: Arc::new(crate::vulkan::Device::new(self, index, create_info).unwrap()),
        })
    }
}
