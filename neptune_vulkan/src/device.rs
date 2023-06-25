use crate::AshInstance;
use ash::vk;
use std::sync::Arc;

pub struct AshRaytracing {
    pub acceleration_structure: ash::extensions::khr::AccelerationStructure,
    pub raytracing_pipeline: ash::extensions::khr::RayTracingPipeline,
}

pub struct AshDevice {
    pub physical: vk::PhysicalDevice,
    pub instance: Arc<AshInstance>,
    pub core: ash::Device,
    pub swapchain: ash::extensions::khr::Swapchain,
    pub full_screen_exclusive: Option<ash::extensions::ext::FullScreenExclusive>,
    pub mesh_shading: Option<ash::extensions::ext::MeshShader>,
    pub raytracing: Option<AshRaytracing>,
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

        let mut synchronization2_features =
            vk::PhysicalDeviceSynchronization2FeaturesKHR::builder()
                .synchronization2(true)
                .build();

        let core = unsafe {
            instance.core.create_device(
                physical_device,
                &vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&queue_create_infos)
                    .enabled_extension_names(&device_extension_names_raw)
                    .push_next(&mut synchronization2_features)
                    .build(),
                None,
            )
        }?;

        let swapchain = ash::extensions::khr::Swapchain::new(&instance.core, &core);

        Ok(Self {
            physical: physical_device,
            instance,
            core,
            swapchain,
            full_screen_exclusive: None,
            mesh_shading: None,
            raytracing: None,
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
