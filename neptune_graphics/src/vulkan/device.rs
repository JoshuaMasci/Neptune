use crate::traits::{DeviceTrait, RenderGraphBuilderTrait};
use crate::vulkan::instance::AshPhysicalDeviceQueues;
use crate::vulkan::Instance;
use crate::{
    BufferDescription, BufferHandle, ComputePipelineDescription, ComputePipelineHandle,
    DeviceCreateInfo, PhysicalDeviceExtensions, PhysicalDeviceFeatures, RasterPipelineDescription,
    RasterPipelineHandle, SamplerDescription, SamplerHandle, SurfaceHandle, SwapchainDescription,
    SwapchainHandle, TextureDescription, TextureHandle,
};
use ash::vk;
use log::trace;
use std::sync::Arc;
use thiserror::Error;

pub struct DropDevice(Arc<ash::Device>);
impl Drop for DropDevice {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_device(None);
            trace!("Drop Device");
        }
    }
}

pub(crate) struct AshDevice {
    #[allow(dead_code)]
    physical_device: vk::PhysicalDevice,
    handle: Arc<ash::Device>,

    primary_queue: vk::Queue,
    compute_queue: Option<vk::Queue>,
    transfer_queue: Option<vk::Queue>,

    swapchain_extension: Arc<ash::extensions::khr::Swapchain>,
    dynamic_rendering_extension: Option<Arc<ash::extensions::khr::DynamicRendering>>,
    mesh_shading_extension: Option<Arc<ash::extensions::ext::MeshShader>>,
    acceleration_structure_extension: Option<Arc<ash::extensions::khr::AccelerationStructure>>,
    ray_tracing_pipeline_extension: Option<Arc<ash::extensions::khr::RayTracingPipeline>>,
}

impl AshDevice {
    fn new(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        queues: &AshPhysicalDeviceQueues,
        extensions: &PhysicalDeviceExtensions,
    ) -> ash::prelude::VkResult<Self> {
        let mut device_extension_names_raw = vec![ash::extensions::khr::Swapchain::name().as_ptr()];

        if extensions.dynamic_rendering {
            device_extension_names_raw
                .push(ash::extensions::khr::DynamicRendering::name().as_ptr());
        }

        if extensions.ray_tracing {
            device_extension_names_raw
                .push(ash::extensions::khr::AccelerationStructure::name().as_ptr());
            device_extension_names_raw
                .push(ash::extensions::khr::RayTracingPipeline::name().as_ptr());
            //Required by AccelerationStructureKHR but it won't be used
            device_extension_names_raw
                .push(ash::extensions::khr::DeferredHostOperations::name().as_ptr());
        }

        if extensions.mesh_shading {
            device_extension_names_raw.push(ash::extensions::ext::MeshShader::name().as_ptr());
        }

        //TODO: check feature support
        let mut synchronization2_features =
            vk::PhysicalDeviceSynchronization2FeaturesKHR::builder()
                .synchronization2(true)
                .build();

        let mut robustness2_features = vk::PhysicalDeviceRobustness2FeaturesEXT::builder()
            .null_descriptor(true)
            .build();
        let mut vulkan1_2_features = vk::PhysicalDeviceVulkan12Features::builder()
            .descriptor_indexing(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_uniform_buffer_update_after_bind(true)
            .descriptor_binding_storage_buffer_update_after_bind(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .descriptor_binding_storage_image_update_after_bind(true)
            .descriptor_binding_update_unused_while_pending(true)
            .build();

        let priorities = &[1.0];
        let mut queue_info = vec![vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queues.primary_queue_family_index)
            .queue_priorities(priorities)
            .build()];

        if let Some(compute_queue_family_index) = queues.compute_queue_family_index {
            queue_info.push(
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(compute_queue_family_index)
                    .queue_priorities(priorities)
                    .build(),
            );
        }

        if let Some(transfer_queue_family_index) = queues.transfer_queue_family_index {
            queue_info.push(
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(transfer_queue_family_index)
                    .queue_priorities(priorities)
                    .build(),
            );
        }

        let device = unsafe {
            instance.create_device(
                physical_device,
                &vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&queue_info)
                    .enabled_extension_names(&device_extension_names_raw)
                    .push_next(&mut synchronization2_features)
                    .push_next(&mut robustness2_features)
                    .push_next(&mut vulkan1_2_features),
                None,
            )
        }?;

        let primary_queue =
            unsafe { device.get_device_queue(queues.primary_queue_family_index, 0) };

        let compute_queue =
            queues
                .compute_queue_family_index
                .map(|compute_queue_family_index| unsafe {
                    device.get_device_queue(compute_queue_family_index, 0)
                });

        let transfer_queue =
            queues
                .transfer_queue_family_index
                .map(|transfer_queue_family_index| unsafe {
                    device.get_device_queue(transfer_queue_family_index, 0)
                });

        let swapchain_extension = Arc::new(ash::extensions::khr::Swapchain::new(instance, &device));

        let dynamic_rendering_extension = if extensions.dynamic_rendering {
            Some(Arc::new(ash::extensions::khr::DynamicRendering::new(
                instance, &device,
            )))
        } else {
            None
        };

        let mesh_shading_extension = if extensions.mesh_shading {
            Some(Arc::new(ash::extensions::ext::MeshShader::new(
                instance, &device,
            )))
        } else {
            None
        };

        let (acceleration_structure_extension, ray_tracing_pipeline_extension) =
            if extensions.ray_tracing {
                (
                    Some(Arc::new(ash::extensions::khr::AccelerationStructure::new(
                        instance, &device,
                    ))),
                    Some(Arc::new(ash::extensions::khr::RayTracingPipeline::new(
                        instance, &device,
                    ))),
                )
            } else {
                (None, None)
            };

        Ok(Self {
            physical_device,
            handle: Arc::new(device),
            primary_queue,
            compute_queue,
            transfer_queue,
            swapchain_extension,
            dynamic_rendering_extension,
            mesh_shading_extension,
            acceleration_structure_extension,
            ray_tracing_pipeline_extension,
        })
    }
}

#[derive(Error, Debug)]
pub(crate) enum DeviceCreateError {
    #[error("Vk error: {0}")]
    VkError(vk::Result),

    #[error("Gpu alloc error: {0}")]
    GpuAllocError(gpu_allocator::AllocationError),
}

pub struct Device {
    drop_device: DropDevice,
    device: Arc<AshDevice>,
}

impl Device {
    pub(crate) fn new(
        instance: &Instance,
        device_index: usize,
        create_info: &DeviceCreateInfo,
    ) -> Result<Self, DeviceCreateError> {
        let physical_device = &instance.physical_devices[device_index];

        //TODO: verify that queue and extensions are supported

        let mut queues = physical_device.queues.clone();

        if !create_info.features.async_compute {
            queues.compute_queue_family_index = None;
        }

        if !create_info.features.async_transfer {
            queues.transfer_queue_family_index = None;
        }

        let ash_device = match AshDevice::new(
            &instance.instance.handle,
            physical_device.handle,
            &queues,
            &create_info.extensions,
        ) {
            Ok(device) => Arc::new(device),
            Err(e) => return Err(DeviceCreateError::VkError(e)),
        };

        Ok(Self {
            drop_device: DropDevice(ash_device.handle.clone()),
            device: ash_device,
        })
    }
}

impl DeviceTrait for Device {
    fn create_buffer(
        &self,
        name: &str,
        description: &BufferDescription,
    ) -> crate::Result<BufferHandle> {
        todo!()
    }

    fn destroy_buffer(&self, handle: BufferHandle) {
        todo!()
    }

    fn create_texture(
        &self,
        name: &str,
        description: &TextureDescription,
    ) -> crate::Result<TextureHandle> {
        todo!()
    }

    fn destroy_texture(&self, handle: TextureHandle) {
        todo!()
    }

    fn create_sampler(
        &self,
        name: &str,
        description: &SamplerDescription,
    ) -> crate::Result<SamplerHandle> {
        todo!()
    }

    fn destroy_sampler(&self, handle: SamplerHandle) {
        todo!()
    }

    fn create_compute_pipeline(
        &self,
        name: &str,
        description: &ComputePipelineDescription,
    ) -> crate::Result<ComputePipelineHandle> {
        todo!()
    }

    fn destroy_compute_pipeline(&self, handle: ComputePipelineHandle) {
        todo!()
    }

    fn create_raster_pipeline(
        &self,
        name: &str,
        description: &RasterPipelineDescription,
    ) -> crate::Result<RasterPipelineHandle> {
        todo!()
    }

    fn destroy_raster_pipeline(&self, handle: RasterPipelineHandle) {
        todo!()
    }

    fn create_swapchain(
        &self,
        name: &str,
        surface: SurfaceHandle,
        description: &SwapchainDescription,
    ) -> crate::Result<SwapchainHandle> {
        todo!()
    }

    fn destroy_swapchain(&self, handle: SwapchainHandle) {
        todo!()
    }

    fn update_swapchain(
        &self,
        handle: SwapchainHandle,
        description: &SwapchainDescription,
    ) -> crate::Result<()> {
        todo!()
    }

    fn begin_frame(&self) -> Box<dyn RenderGraphBuilderTrait> {
        todo!()
    }

    fn end_frame(&self, render_graph: Box<dyn RenderGraphBuilderTrait>) -> crate::Result<()> {
        todo!()
    }
}
