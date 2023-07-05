use crate::AshInstance;
use ash::vk;
use std::sync::Arc;

pub struct AshRaytracing {
    pub acceleration_structure: ash::extensions::khr::AccelerationStructure,
    pub raytracing_pipeline: ash::extensions::khr::RayTracingPipeline,
}

#[derive(Clone, Debug)]
pub struct AshQueue {
    pub family_index: u32,
    pub handle: vk::Queue,
    pub flags: vk::QueueFlags,
}

pub struct AshDevice {
    pub physical: vk::PhysicalDevice,
    pub instance: Arc<AshInstance>,
    pub core: ash::Device,
    pub swapchain: ash::extensions::khr::Swapchain,
    pub full_screen_exclusive: Option<ash::extensions::ext::FullScreenExclusive>,
    pub mesh_shading: Option<ash::extensions::ext::MeshShader>,
    pub raytracing: Option<AshRaytracing>,
    pub queues: Vec<AshQueue>,
}

impl AshDevice {
    pub fn new(
        instance: Arc<AshInstance>,
        physical_device: vk::PhysicalDevice,
        queues: &[u32],
    ) -> ash::prelude::VkResult<Self> {
        let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = queues
            .iter()
            .map(|family_index| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(*family_index)
                    .queue_priorities(&[1.0])
                    .build()
            })
            .collect();

        let device_extension_names_raw = vec![ash::extensions::khr::Swapchain::name().as_ptr()];

        let mut vulkan_1_3_features = vk::PhysicalDeviceVulkan13Features::builder()
            .synchronization2(true)
            .dynamic_rendering(true);

        let core = unsafe {
            instance.core.create_device(
                physical_device,
                &vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&queue_create_infos)
                    .enabled_extension_names(&device_extension_names_raw)
                    .push_next(&mut vulkan_1_3_features)
                    .build(),
                None,
            )
        }?;

        let swapchain = ash::extensions::khr::Swapchain::new(&instance.core, &core);

        let queue_family_properties = unsafe {
            instance
                .core
                .get_physical_device_queue_family_properties(physical_device)
        };

        let queues = queues
            .iter()
            .map(|&family_index| AshQueue {
                family_index,
                handle: unsafe { core.get_device_queue(family_index, 0) },
                flags: queue_family_properties[family_index as usize].queue_flags,
            })
            .collect();

        Ok(Self {
            physical: physical_device,
            instance,
            core,
            swapchain,
            full_screen_exclusive: None,
            mesh_shading: None,
            raytracing: None,
            queues,
        })
    }
}

impl Drop for AshDevice {
    fn drop(&mut self) {
        unsafe {
            self.core.destroy_device(None);
        }
    }
}
