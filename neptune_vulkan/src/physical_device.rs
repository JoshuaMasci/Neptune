use crate::device::DeviceSettings;
use crate::instance::AshInstance;
use crate::{Device, SurfaceHandle, VulkanError};
use ash::vk;
use log::error;
use std::ffi::{c_char, CStr};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

fn c_str_to_string(c_str: &[c_char]) -> String {
    unsafe {
        CStr::from_ptr(c_str.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}

fn find_queue_index(
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

fn sum_memory_heaps(
    memory_heaps: &[vk::MemoryHeap],
    contains_flags: vk::MemoryHeapFlags,
    exclude_flags: vk::MemoryHeapFlags,
) -> usize {
    memory_heaps
        .iter()
        .filter(|&memory_heap| {
            memory_heap.flags.contains(contains_flags)
                && !memory_heap.flags.intersects(exclude_flags)
        })
        .map(|&memory_heap| memory_heap.size as usize)
        .sum()
}
fn supports_extension(extension_list: &[vk::ExtensionProperties], name: &CStr) -> bool {
    extension_list.iter().any(|extension_properties| {
        name == unsafe { CStr::from_ptr(extension_properties.extension_name.as_ptr()) }
    })
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum PhysicalDeviceVendor {
    Amd,
    Arm,
    ImgTec,
    Intel,
    Nvidia,
    Qualcomm,
    Broadcom,
    Unknown { vendor_id: u32 },
}

impl PhysicalDeviceVendor {
    pub(crate) fn from_vulkan(vendor_id: u32) -> Self {
        // List from here: https://www.reddit.com/r/vulkan/comments/4ta9nj/is_there_a_comprehensive_list_of_the_names_and/
        //TODO: find a more complete list?
        match vendor_id {
            0x1002 => Self::Amd,
            0x10DE => Self::Nvidia,
            0x8086 => Self::Intel,
            0x1010 => Self::ImgTec,
            0x13B5 => Self::Arm,
            0x5132 => Self::Qualcomm,
            0x14e4 => Self::Broadcom,
            vendor_id => Self::Unknown { vendor_id },
        }
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum PhysicalDeviceType {
    Integrated,
    Discrete,
    Unknown,
}

impl PhysicalDeviceType {
    pub(crate) fn from_vulkan(device_type: vk::PhysicalDeviceType) -> Self {
        match device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => Self::Discrete,
            vk::PhysicalDeviceType::INTEGRATED_GPU => Self::Integrated,
            _ => Self::Unknown,
        }
    }
}

fn get_driver_version(driver_version: u32, vendor: PhysicalDeviceVendor) -> String {
    // List from https://github.com/SaschaWillems/vulkan.gpuinfo.org/blob/f6e27d8446f81c273b6d2688caf054c7a2314d32/includes/functions.php#L414C1-L415C1
    match vendor {
        #[cfg(target_os = "windows")]
        PhysicalDeviceVendor::Intel => {
            // Intel + Windows
            format!("{}.{}", (driver_version >> 14), driver_version & 0x3fff)
        }
        PhysicalDeviceVendor::Nvidia => {
            format!(
                "{}.{}.{}.{}",
                (driver_version >> 22) & 0x3ff,
                (driver_version >> 14) & 0x0ff,
                (driver_version >> 6) & 0x0ff,
                driver_version & 0x003f
            )
        }
        PhysicalDeviceVendor::Broadcom => {
            // Version encoded as human-readable (10000 * major + 100 * minor)
            let major = driver_version / 10_000;
            let minor = (driver_version % 10_000) / 100;
            format!("{}.{}", major, minor)
        }
        PhysicalDeviceVendor::ImgTec => {
            // For production drivers, driverVersion is a monotonic integer
            // changeset number that a driver release was built from.
            //
            // The VK_KHR_driver_properties driverInfo field provides more information
            // such as the major/minor release branch, 1.10, 1.11, etc.
            //
            // Non-production builds are automatically given a made-up version starting
            // from 500,000,000 and can be ignored/formatted separately to not clash.
            if driver_version > 500_000_000 {
                format!("0.0.{}", driver_version)
            } else {
                format!("{}", driver_version)
            }
        }
        _ => {
            // Use Vulkan version conventions if vendor mapping is not available
            format!(
                "{}.{}.{}",
                driver_version >> 22,
                (driver_version >> 12) & 0x3ff,
                driver_version & 0xfff,
            )
        }
    }
}

#[derive(Clone, Debug)]
pub struct PhysicalDeviceInfo {
    pub name: String,
    pub device_id: u32,
    pub api_version: [u32; 4],
    pub vendor: PhysicalDeviceVendor,
    pub device_type: PhysicalDeviceType,
}

#[derive(Clone, Debug)]
pub struct PhysicalDeviceDriverInfo {
    pub id: String,
    pub name: String,
    pub info: String,
    pub version: String,
}

#[derive(Clone, Debug)]
pub struct PhysicalDeviceMemoryInfo {
    pub device_local_bytes: usize,
    pub host_visible_bytes: usize,
}

#[derive(Clone)]
pub struct PhysicalDeviceQueueInfo {
    pub graphics_queue_family_index: Option<u32>,
    pub compute_queue_family_index: Option<u32>,
    pub transfer_queue_family_index: Option<u32>,
}

impl Debug for PhysicalDeviceQueueInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhysicalDeviceQueueInfo")
            .field(
                "graphics_support",
                &self.graphics_queue_family_index.is_some(),
            )
            .field(
                "async_compute_support",
                &self.compute_queue_family_index.is_some(),
            )
            .field(
                "async_transfer_support",
                &self.transfer_queue_family_index.is_some(),
            )
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct PhysicalDeviceExtensionInfo {
    pub raytracing_support: bool,
    pub mesh_shader_support: bool,
}

#[derive(Clone)]
pub struct PhysicalDevice {
    pub(crate) instance: Arc<AshInstance>,
    pub(crate) handle: vk::PhysicalDevice,

    //TODO: supported extensions list (Raytracing + Mesh Shading)
    //TODO: support 2 transfer queues?
    pub info: PhysicalDeviceInfo,
    pub driver: PhysicalDeviceDriverInfo,
    pub memory: PhysicalDeviceMemoryInfo,
    pub queue: PhysicalDeviceQueueInfo,
    pub extension: PhysicalDeviceExtensionInfo,
}

impl PhysicalDevice {
    pub(crate) fn new(instance: Arc<AshInstance>, physical_device: vk::PhysicalDevice) -> Self {
        let (device_properties, driver_properties) = {
            let mut driver_properties = vk::PhysicalDeviceDriverProperties::default();
            let mut properties2 =
                vk::PhysicalDeviceProperties2::builder().push_next(&mut driver_properties);
            unsafe {
                instance
                    .core
                    .get_physical_device_properties2(physical_device, &mut properties2);
            };
            (properties2.properties, driver_properties)
        };

        let info = PhysicalDeviceInfo {
            name: c_str_to_string(&device_properties.device_name),
            device_id: device_properties.device_id,
            api_version: [
                vk::api_version_variant(device_properties.api_version),
                vk::api_version_major(device_properties.api_version),
                vk::api_version_minor(device_properties.api_version),
                vk::api_version_patch(device_properties.api_version),
            ],
            vendor: PhysicalDeviceVendor::from_vulkan(device_properties.vendor_id),
            device_type: PhysicalDeviceType::from_vulkan(device_properties.device_type),
        };

        let driver = PhysicalDeviceDriverInfo {
            id: format!("{:?}", driver_properties.driver_id),
            name: c_str_to_string(&driver_properties.driver_name),
            info: c_str_to_string(&driver_properties.driver_info),
            version: get_driver_version(device_properties.driver_version, info.vendor),
        };

        let device_memory = unsafe {
            instance
                .core
                .get_physical_device_memory_properties(physical_device)
        };
        let memory_heaps = &device_memory.memory_heaps[0..device_memory.memory_heap_count as usize];
        let memory = PhysicalDeviceMemoryInfo {
            device_local_bytes: sum_memory_heaps(
                memory_heaps,
                vk::MemoryHeapFlags::DEVICE_LOCAL,
                vk::MemoryHeapFlags::empty(),
            ),
            host_visible_bytes: sum_memory_heaps(
                memory_heaps,
                vk::MemoryHeapFlags::empty(),
                vk::MemoryHeapFlags::DEVICE_LOCAL,
            ),
        };

        let queue_family_properties = unsafe {
            instance
                .core
                .get_physical_device_queue_family_properties(physical_device)
        };

        //TODO: for the moment, only choose a queue family that supports all operations
        // This will work for most desktop GPU's, as they will have this type of queue family
        // However I would still like to make a more robust queue family selection system for the other GPU's
        let queue = PhysicalDeviceQueueInfo {
            graphics_queue_family_index: find_queue_index(
                &queue_family_properties,
                vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER,
                vk::QueueFlags::empty(),
            ),
            compute_queue_family_index: find_queue_index(
                &queue_family_properties,
                vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER,
                vk::QueueFlags::GRAPHICS,
            ),
            transfer_queue_family_index: find_queue_index(
                &queue_family_properties,
                vk::QueueFlags::TRANSFER,
                vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE,
            ),
        };

        let extension_list = unsafe {
            instance
                .core
                .enumerate_device_extension_properties(physical_device)
        }
        .unwrap_or_default();

        let extension = PhysicalDeviceExtensionInfo {
            raytracing_support: supports_extension(
                &extension_list,
                ash::extensions::khr::AccelerationStructure::name(),
            ) && supports_extension(
                &extension_list,
                ash::extensions::khr::RayTracingPipeline::name(),
            ),
            mesh_shader_support: supports_extension(
                &extension_list,
                ash::extensions::ext::MeshShader::name(),
            ),
        };

        Self {
            instance,
            handle: physical_device,
            info,
            driver,
            memory,
            queue,
            extension,
        }
    }

    pub fn supports_graphics(&self) -> bool {
        self.queue.graphics_queue_family_index.is_some()
    }

    pub fn supports_async_compute(&self) -> bool {
        self.queue.compute_queue_family_index.is_some()
    }

    pub fn supports_async_transfer(&self) -> bool {
        self.queue.transfer_queue_family_index.is_some()
    }

    pub fn supports_surface(&self, surface_handle: SurfaceHandle) -> bool {
        if let Some(graphics_queue_family_index) = self.queue.graphics_queue_family_index {
            if let Some(surface) = self.instance.surface_list.get(surface_handle.0) {
                unsafe {
                    match self.instance.surface.get_physical_device_surface_support(
                        self.handle,
                        graphics_queue_family_index,
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
        } else {
            false
        }
    }

    pub fn create_device(self, settings: DeviceSettings) -> Result<Device, VulkanError> {
        Device::new(self.instance.clone(), self, settings)
    }
}

impl Debug for PhysicalDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhysicalDevice")
            .field("handle", &self.handle)
            .field("info", &self.info)
            .field("driver", &self.driver)
            .field("memory", &self.memory)
            .field("queue", &self.queue)
            .field("extension", &self.extension)
            .finish()
    }
}
