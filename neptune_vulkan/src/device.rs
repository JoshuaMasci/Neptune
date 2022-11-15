use crate::descriptor_set::{DescriptorPool, DescriptorSetLayout, DescriptorSetLayoutPool};
use crate::{Buffer, Image, ImageView, MemoryLocation, Sampler};
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
    //TODO: Add VRam amount, Other Device Properties?
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
    descriptor_set_layout_pool: Arc<DescriptorSetLayoutPool>,
    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    device: Arc<AshDevice>,

    surface_ext: Arc<ash::extensions::khr::Surface>,
    swapchain_ext: Arc<ash::extensions::khr::Swapchain>,

    info: DeviceInfo,
    physical_device: vk::PhysicalDevice,

    graphics_queue: vk::Queue,
}

impl Device {
    pub(crate) fn new(
        instance: &ash::Instance,
        physical_device: &PhysicalDevice,
        surface_ext: Arc<ash::extensions::khr::Surface>,
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

        let swapchain_ext = Arc::new(ash::extensions::khr::Swapchain::new(instance, &device.0));

        let descriptor_set_layout_pool = Arc::new(DescriptorSetLayoutPool::new(&device));

        Ok(Self {
            descriptor_set_layout_pool,
            info: physical_device.device_info.clone(),
            physical_device: physical_device.handle,
            device,
            surface_ext,
            swapchain_ext,
            allocator,
            graphics_queue,
        })
    }

    pub fn info(&self) -> DeviceInfo {
        self.info.clone()
    }

    pub fn create_buffer(
        &self,
        _name: &str,
        create_info: &vk::BufferCreateInfo,
        memory_location: MemoryLocation,
    ) -> crate::Result<Arc<Buffer>> {
        Buffer::new(
            self.device.clone(),
            self.allocator.clone(),
            create_info,
            memory_location,
        )
        .map(Arc::new)
    }

    pub fn create_image(
        &self,
        _name: &str,
        create_info: &vk::ImageCreateInfo,
        memory_location: MemoryLocation,
    ) -> crate::Result<Arc<Image>> {
        Image::new(
            self.device.clone(),
            self.allocator.clone(),
            create_info,
            memory_location,
        )
        .map(Arc::new)
    }

    pub fn create_image_view(
        &self,
        _name: &str,
        image: Arc<Image>,
        create_info: &vk::ImageViewCreateInfo,
    ) -> crate::Result<Arc<ImageView>> {
        ImageView::new(image, create_info).map(Arc::new)
    }

    pub fn create_sampler(
        &self,
        _name: &str,
        create_info: &vk::SamplerCreateInfo,
    ) -> crate::Result<Arc<Sampler>> {
        Sampler::new(self.device.clone(), create_info).map(Arc::new)
    }

    pub fn create_descriptor_pool<T: DescriptorSetLayout>(
        &self,
        max_sets: u32,
    ) -> crate::Result<Arc<DescriptorPool<T>>> {
        DescriptorPool::new(&self.device, &self.descriptor_set_layout_pool, max_sets).map(Arc::new)
    }
}
