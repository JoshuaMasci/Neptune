use crate::traits::{DeviceTrait, RenderGraphBuilderTrait};
use crate::vulkan::buffer::AshBuffer;
use crate::vulkan::image::AshImage;
use crate::vulkan::instance::{AshInstance, AshPhysicalDeviceQueues};
use crate::vulkan::sampler::AshSampler;
use crate::vulkan::{AshBufferHandle, AshSamplerHandle, AshTextureHandle, Instance};
use crate::{
    BufferDescription, BufferHandle, ComputePipelineDescription, ComputePipelineHandle,
    DeviceCreateInfo, PhysicalDeviceExtensions, RasterPipelineDescription, RasterPipelineHandle,
    SamplerDescription, SamplerHandle, SurfaceHandle, SwapchainDescription, SwapchainHandle,
    TextureDescription, TextureHandle,
};
use ash::prelude::VkResult;
use ash::vk;
use gpu_allocator::MemoryLocation;
use slotmap::{KeyData, SlotMap};
use std::sync::{Arc, Mutex};
use thiserror::Error;

pub struct DropDevice(Arc<ash::Device>);
impl Drop for DropDevice {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_device(None);
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
    device: AshDevice,
    buffers: Mutex<SlotMap<AshBufferHandle, Arc<AshBuffer>>>,
    textures: Mutex<SlotMap<AshTextureHandle, Arc<AshImage>>>,
    samplers: Mutex<SlotMap<AshSamplerHandle, Arc<AshSampler>>>,

    allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
    #[allow(unused)]
    drop_device: DropDevice,
    instance: Arc<AshInstance>,
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

        let device = match AshDevice::new(
            &instance.instance.handle,
            physical_device.handle,
            &queues,
            &create_info.extensions,
        ) {
            Ok(device) => device,
            Err(e) => return Err(DeviceCreateError::VkError(e)),
        };

        let allocator = match gpu_allocator::vulkan::Allocator::new(
            &gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.instance.handle.clone(),
                device: (*device.handle).clone(),
                physical_device: device.physical_device,
                debug_settings: gpu_allocator::AllocatorDebugSettings::default(),
                buffer_device_address: false,
            },
        ) {
            Ok(allocator) => Arc::new(Mutex::new(allocator)),
            Err(e) => return Err(DeviceCreateError::GpuAllocError(e)),
        };

        Ok(Self {
            buffers: Mutex::new(SlotMap::with_key()),
            textures: Mutex::new(SlotMap::with_key()),
            samplers: Mutex::new(SlotMap::with_key()),
            allocator,
            instance: instance.instance.clone(),
            drop_device: DropDevice(device.handle.clone()),
            device,
        })
    }
}

impl DeviceTrait for Device {
    fn create_buffer(
        &self,
        name: &str,
        description: &BufferDescription,
    ) -> crate::Result<BufferHandle> {
        let create_info = vk::BufferCreateInfo::builder()
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .usage(description.usage.to_vk())
            .size(description.size)
            .build();

        let buffer = match AshBuffer::new(
            self.device.handle.clone(),
            self.allocator.clone(),
            name,
            &create_info,
            MemoryLocation::GpuOnly,
        ) {
            Ok(buffer) => buffer,
            Err(_e) => return Err(crate::Error::TempError),
        };

        if let Some(debug_utils) = &self.instance.debug_utils {
            let _ = debug_utils.set_object_name(self.device.handle.handle(), buffer.handle, name);
        }

        let buffer_handle = self.buffers.lock().unwrap().insert(Arc::new(buffer));
        Ok(buffer_handle.0.as_ffi())
    }

    fn destroy_buffer(&self, handle: BufferHandle) {
        let buffer_handle = AshBufferHandle::from(KeyData::from_ffi(handle));
        let _ = self.buffers.lock().unwrap().remove(buffer_handle);
    }

    fn create_texture(
        &self,
        name: &str,
        description: &TextureDescription,
    ) -> crate::Result<TextureHandle> {
        let is_color = description.format.is_color();
        let format = description.format.to_vk();

        let create_info = vk::ImageCreateInfo::builder()
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .usage(
                description
                    .usage
                    .to_vk(is_color, description.sampler.is_some()),
            )
            .format(format)
            .extent(vk::Extent3D {
                width: description.size[0],
                height: description.size[1],
                depth: 1,
            })
            .image_type(vk::ImageType::TYPE_2D)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .flags(vk::ImageCreateFlags::empty())
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .build();
        let mut image = match AshImage::new(
            self.device.handle.clone(),
            self.allocator.clone(),
            name,
            &create_info,
            MemoryLocation::GpuOnly,
        ) {
            Ok(image) => image,
            Err(_e) => return Err(crate::Error::TempError),
        };

        let view_create_info = vk::ImageViewCreateInfo::builder()
            .image(image.handle)
            .format(format)
            .view_type(vk::ImageViewType::TYPE_2D)
            .components(vk::ComponentMapping::default())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: if is_color {
                    vk::ImageAspectFlags::COLOR
                } else {
                    vk::ImageAspectFlags::DEPTH
                },
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build();
        if let Err(_e) = image.create_view(&view_create_info) {
            return Err(crate::Error::TempError);
        }

        if let Some(debug_utils) = &self.instance.debug_utils {
            let _ = debug_utils.set_object_name(self.device.handle.handle(), image.handle, name);
            let _ =
                debug_utils.set_object_name(self.device.handle.handle(), image.view_handle, name);
        }

        let texture_handle = self.textures.lock().unwrap().insert(Arc::new(image));
        Ok(texture_handle.0.as_ffi())
    }

    fn destroy_texture(&self, handle: TextureHandle) {
        let texture_handle = AshTextureHandle::from(KeyData::from_ffi(handle));
        let _ = self.textures.lock().unwrap().remove(texture_handle);
    }

    fn create_sampler(
        &self,
        name: &str,
        description: &SamplerDescription,
    ) -> crate::Result<SamplerHandle> {
        let sampler = match AshSampler::new(self.device.handle.clone(), &description.to_vk()) {
            Ok(sampler) => sampler,
            Err(_e) => return Err(crate::Error::TempError),
        };

        if let Some(debug_utils) = &self.instance.debug_utils {
            let _ = debug_utils.set_object_name(self.device.handle.handle(), sampler.handle, name);
        }

        let sampler_handle = self.samplers.lock().unwrap().insert(Arc::new(sampler));
        Ok(sampler_handle.0.as_ffi())
    }

    fn destroy_sampler(&self, handle: SamplerHandle) {
        let sampler_handle = AshSamplerHandle::from(KeyData::from_ffi(handle));
        let _ = self.samplers.lock().unwrap().remove(sampler_handle);
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
