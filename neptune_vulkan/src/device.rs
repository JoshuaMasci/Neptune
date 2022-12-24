use crate::resource_manager::ResourceManager;
use crate::sampler::{Sampler, SamplerCreateInfo};
use crate::texture::{Texture, TextureBindingType, TextureUsage};
use crate::{Buffer, BufferBindingType, BufferUsage, MemoryLocation};
use crate::{Error, PhysicalDevice};
use ash::vk;
use std::ffi::CStr;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Integrated,
    Discrete,
    Unknown,
}

impl DeviceType {
    fn from_vk(device_type: vk::PhysicalDeviceType) -> Self {
        match device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => Self::Discrete,
            vk::PhysicalDeviceType::INTEGRATED_GPU => Self::Integrated,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceVendor {
    Amd,
    Arm,
    ImgTec,
    Intel,
    Nvidia,
    Qualcomm,
    Unknown(u32),
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

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub vendor: DeviceVendor,
    pub device_type: DeviceType,
}

impl DeviceInfo {
    pub(crate) fn new(physical_device_properties: vk::PhysicalDeviceProperties) -> Self {
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

#[derive(Clone)]
pub struct AshDevice(ash::Device);
impl AshDevice {
    fn new(device: ash::Device) -> Self {
        Self(device)
    }
}

impl Deref for AshDevice {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for AshDevice {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_device(None);
            trace!("Drop Device");
        }
    }
}

pub struct Device {
    resource_manager: Arc<Mutex<ResourceManager>>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    device: Arc<AshDevice>,

    info: DeviceInfo,
    physical_device: vk::PhysicalDevice,

    graphics_queue: vk::Queue,
}

impl Device {
    pub(crate) fn new(
        instance: &ash::Instance,
        physical_device: &PhysicalDevice,
    ) -> crate::Result<Self> {
        let device_extension_names_raw = vec![ash::extensions::khr::Swapchain::name().as_ptr()];

        let mut synchronization2_features =
            vk::PhysicalDeviceSynchronization2FeaturesKHR::builder()
                .synchronization2(true)
                .build();

        let priorities = &[1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(physical_device.graphics_queue_family_index)
            .queue_priorities(priorities)
            .build()];

        let device = match unsafe {
            instance.create_device(
                physical_device.handle,
                &vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&queue_info)
                    .enabled_extension_names(&device_extension_names_raw)
                    .push_next(&mut synchronization2_features),
                None,
            )
        } {
            Ok(device) => device,
            Err(e) => return Err(Error::VkError(e)),
        };

        let graphics_queue =
            unsafe { device.get_device_queue(physical_device.graphics_queue_family_index, 0) };

        let device = Arc::new(AshDevice::new(device));

        let allocator = match gpu_allocator::vulkan::Allocator::new(
            &gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.clone(),
                device: (**device).clone(),
                physical_device: physical_device.handle,
                debug_settings: gpu_allocator::AllocatorDebugSettings::default(),
                buffer_device_address: false,
            },
        ) {
            Ok(allocator) => Arc::new(Mutex::new(allocator)),
            Err(e) => return Err(Error::GpuAllocError(e)),
        };

        const FRAMES_IN_FLIGHT_COUNT: usize = 3;
        let resource_manager = Arc::new(Mutex::new(ResourceManager::new(
            FRAMES_IN_FLIGHT_COUNT,
            device.clone(),
            allocator.clone(),
        )));

        Ok(Self {
            info: physical_device.device_info.clone(),
            physical_device: physical_device.handle,
            device,
            allocator,
            resource_manager,
            graphics_queue,
        })
    }

    pub fn info(&self) -> DeviceInfo {
        self.info.clone()
    }

    pub fn create_buffer(
        &self,
        name: &str,
        usage: BufferUsage,
        binding: BufferBindingType,
        size: u64,
    ) -> crate::Result<Buffer> {
        let create_info = crate::buffer::get_vk_buffer_create_info(usage, binding, size);
        crate::buffer::AshBuffer::create_buffer(
            &self.device,
            &self.allocator,
            &create_info,
            MemoryLocation::GpuOnly,
        )
        .map(|buffer| Buffer {
            buffer,
            resource_manager: self.resource_manager.clone(),
        })
    }

    pub fn create_buffer_with_data(
        &self,
        name: &str,
        usage: BufferUsage,
        binding: BufferBindingType,
        data: &[u8],
    ) -> crate::Result<Buffer> {
        let create_info =
            crate::buffer::get_vk_buffer_create_info(usage, binding, data.len() as u64);
        crate::buffer::AshBuffer::create_buffer(
            &self.device,
            &self.allocator,
            &create_info,
            MemoryLocation::GpuOnly,
        )
        .map(|buffer| Buffer {
            buffer,
            resource_manager: self.resource_manager.clone(),
        })
    }

    pub fn create_texture(
        &self,
        name: &str,
        usage: TextureUsage,
        bindings: TextureBindingType,
        format: vk::Format,
        size: [u32; 2],
    ) -> crate::Result<Texture> {
        let create_info =
            crate::texture::get_vk_texture_2d_create_info(usage, bindings, format, size);
        crate::texture::AshTexture::create_texture(
            &self.device,
            &self.allocator,
            &create_info,
            MemoryLocation::GpuOnly,
        )
        .map(|texture| Texture {
            texture,
            resource_manager: self.resource_manager.clone(),
        })
    }

    pub fn create_texture_with_data(
        &self,
        name: &str,
        usage: TextureUsage,
        bindings: TextureBindingType,
        format: vk::Format,
        size: [u32; 2],
        data: &[u8],
    ) -> crate::Result<Texture> {
        let create_info =
            crate::texture::get_vk_texture_2d_create_info(usage, bindings, format, size);
        crate::texture::AshTexture::create_texture(
            &self.device,
            &self.allocator,
            &create_info,
            MemoryLocation::GpuOnly,
        )
        .map(|texture| Texture {
            texture,
            resource_manager: self.resource_manager.clone(),
        })
    }

    pub fn create_sampler(
        &self,
        name: &str,
        sampler_create_info: &SamplerCreateInfo,
    ) -> crate::Result<Sampler> {
        crate::sampler::AshSampler::create_sampler(&self.device, sampler_create_info).map(
            |sampler| Sampler {
                sampler,
                resource_manager: self.resource_manager.clone(),
            },
        )
    }
}
