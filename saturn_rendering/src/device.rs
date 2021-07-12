use std::mem::ManuallyDrop;

use ash::*;
use gpu_allocator::*;

use ash::version::DeviceV1_0;
use ash::version::InstanceV1_0;

pub(crate) struct InternalDevice {
    pub pdevice: vk::PhysicalDevice,
    pub device: ash::Device,
    pub allocator: ManuallyDrop<VulkanAllocator>,
    pub graphics_queue: vk::Queue,
}

impl InternalDevice {
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
            device,
            allocator: ManuallyDrop::new(allocator),
            graphics_queue,
        }
    }
}

impl Drop for InternalDevice {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            ManuallyDrop::drop(&mut self.allocator);
            self.device.destroy_device(None);
        }
    }
}
