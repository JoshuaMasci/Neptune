use ash::*;
use gpu_allocator::*;

use ash::version::DeviceV1_0;
use ash::version::InstanceV1_0;

struct DeviceDrop(ash::Device);

impl Drop for DeviceDrop {
    fn drop(&mut self) {
        unsafe { self.0.destroy_device(None) };
    }
}

pub struct Device {
    pdevice: vk::PhysicalDevice,
    allocator: VulkanAllocator,
    device: DeviceDrop,
    graphics_queue: vk::Queue,
}

impl Device {
    pub(crate) fn new(
        instance: ash::Instance,
        pdevice: ash::vk::PhysicalDevice,
        graphics_queue_index: u32,
    ) -> Self {
        let device_extension_names_raw = [extensions::khr::Swapchain::name().as_ptr()];
        let features = vk::PhysicalDeviceFeatures {
            ..Default::default()
        };
        let priorities = [1.0];

        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_index)
            .queue_priorities(&priorities)
            .build()];

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features);

        let device: ash::Device = unsafe {
            instance
                .create_device(pdevice, &device_create_info, None)
                .unwrap()
        };

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_index, 0) };

        let allocator = VulkanAllocator::new(&VulkanAllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device: pdevice,
            debug_settings: Default::default(),
        });

        Self {
            pdevice,
            device: DeviceDrop(device),
            allocator,
            graphics_queue,
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.device.0.device_wait_idle().unwrap();
        }
    }
}
