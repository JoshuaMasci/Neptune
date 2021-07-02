use ash;
use ash::vk;
use gpu_allocator::*;

use std::sync::Arc;

struct Device {
    pub instance: ash::Instance,
    pub physical_device: ash::vk::PhysicalDevice,

    pub device: ash::Device,
    pub allocator: Arc<VulkanAllocator>,

    pub present_queue: vk::Queue,
    pub graphics_queue: vk::Queue,
    pub compute_queue: vk::Queue,
    pub transfer_queue: vk::Queue,
}

impl Device {
    pub fn new(instance: ash::Instance, physical_device: vk::PhysicalDevice) -> Self {
        //TODO

        Self {
            instance,
            physical_device,

            device: Default::default(),
            allocator: Default::default(),

            present_queue: vk::Queue::null(),
            graphics_queue: vk::Queue::null(),
            compute_queue: vk::Queue::null(),
            transfer_queue: vk::Queue::null(),
        }
    }
}
