use crate::traits::{DeviceTrait, RenderGraphBuilderTrait};
use crate::vulkan::buffer::AshBuffer;
use crate::vulkan::image::AshImage;
use crate::vulkan::instance::{AshInstance, AshPhysicalDeviceQueues, AshSurfaceSwapchains};
use crate::vulkan::render_graph_builder::AshRenderGraphBuilder;
use crate::vulkan::sampler::AshSampler;
use crate::vulkan::swapchain::{AshSwapchain, SwapchainConfig};
use crate::vulkan::{
    AshBufferHandle, AshSamplerHandle, AshSurfaceHandle, AshTextureHandle, Instance,
};
use crate::{
    BufferDescription, BufferHandle, ComputeDispatch, ComputePipeline, ComputePipelineDescription,
    ComputePipelineHandle, DeviceCreateInfo, PhysicalDeviceExtensions, Queue, RasterCommand,
    RasterPassDescription, RasterPipelineDescription, RasterPipelineHandle, SamplerDescription,
    SamplerHandle, ShaderResourceAccess, SurfaceHandle, SwapchainDescription, TextureDescription,
    TextureHandle, TextureUsage, Transfer, TransientTexture,
};
use ash::vk;
use gpu_allocator::MemoryLocation;
use log::warn;
use slotmap::{KeyData, SlotMap};
use std::sync::{Arc, Mutex};
use thiserror::Error;

pub struct DropDevice(Arc<ash::Device>);
impl Drop for DropDevice {
    fn drop(&mut self) {
        warn!("Dropping Device");

        unsafe {
            self.0.destroy_device(None);
        }
    }
}

#[derive(Clone)]
pub(crate) struct AshQueue {
    device: Arc<ash::Device>,
    pub(crate) queue: vk::Queue,
    pub(crate) command_pool: vk::CommandPool,
}

impl AshQueue {
    fn new(device: Arc<ash::Device>, queue_family_index: u32) -> ash::prelude::VkResult<Self> {
        Ok(unsafe {
            Self {
                queue: device.get_device_queue(queue_family_index, 0),
                command_pool: device.create_command_pool(
                    &vk::CommandPoolCreateInfo::builder()
                        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                        .queue_family_index(queue_family_index),
                    None,
                )?,
                device,
            }
        })
    }
}

impl Drop for AshQueue {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);
        }
    }
}

pub(crate) struct AshDevice {
    #[allow(dead_code)]
    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) handle: Arc<ash::Device>,

    pub(crate) primary_queue: AshQueue,
    pub(crate) compute_queue: Option<AshQueue>,
    pub(crate) transfer_queue: Option<AshQueue>,

    pub(crate) swapchain_extension: Arc<ash::extensions::khr::Swapchain>,
    pub(crate) dynamic_rendering_extension: Option<Arc<ash::extensions::khr::DynamicRendering>>,
    pub(crate) mesh_shading_extension: Option<Arc<ash::extensions::ext::MeshShader>>,
    pub(crate) acceleration_structure_extension:
        Option<Arc<ash::extensions::khr::AccelerationStructure>>,
    pub(crate) ray_tracing_pipeline_extension:
        Option<Arc<ash::extensions::khr::RayTracingPipeline>>,
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

        let handle = Arc::new(device.clone());

        let primary_queue = AshQueue::new(handle.clone(), queues.primary_queue_family_index)?;

        let mut compute_queue: Option<AshQueue> = None;
        if let Some(compute_queue_family_index) = queues.compute_queue_family_index {
            compute_queue = Some(AshQueue::new(handle.clone(), compute_queue_family_index)?);
        }

        let mut transfer_queue: Option<AshQueue> = None;
        if let Some(transfer_queue_family_index) = queues.transfer_queue_family_index {
            transfer_queue = Some(AshQueue::new(handle.clone(), transfer_queue_family_index)?);
        }

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
    device: Arc<AshDevice>,
    buffers: Mutex<SlotMap<AshBufferHandle, Arc<AshBuffer>>>,
    textures: Mutex<SlotMap<AshTextureHandle, Arc<AshImage>>>,
    samplers: Mutex<SlotMap<AshSamplerHandle, Arc<AshSampler>>>,

    surfaces_swapchains: Arc<Mutex<SlotMap<AshSurfaceHandle, AshSurfaceSwapchains>>>,

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
            surfaces_swapchains: instance.surfaces_swapchains.clone(),
            allocator,
            instance: instance.instance.clone(),
            drop_device: DropDevice(device.handle.clone()),
            device: Arc::new(device),
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

        let usage = description.usage
            | if description.sampler.is_some() {
                TextureUsage::SAMPLED
            } else {
                TextureUsage::empty()
            };

        let create_info = vk::ImageCreateInfo::builder()
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .usage(usage.to_vk(is_color))
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

    fn configure_swapchain(
        &self,
        surface_handle: SurfaceHandle,
        description: &SwapchainDescription,
    ) -> crate::Result<()> {
        let surface_handle = AshSurfaceHandle::from(KeyData::from_ffi(surface_handle));

        let swapchain_config = SwapchainConfig {
            image_count: 3, //TODO: choose this
            format: vk::SurfaceFormatKHR {
                format: description.surface_format.format.to_vk(),
                color_space: description.surface_format.color_space.to_vk(),
            },
            present_mode: description.present_mode.to_vk(),
            usage: description.usage.to_vk(true),
            composite_alpha: description.composite_alpha.to_vk(),
        };

        let mut surfaces_swapchains = self.surfaces_swapchains.lock().unwrap();
        if let Some(surface_swapchains) = surfaces_swapchains.get_mut(surface_handle) {
            if let Some(swapchain) = surface_swapchains
                .swapchains
                .get_mut(&self.device.physical_device)
            {
                if let Err(_e) = swapchain.update_config(swapchain_config) {
                    return Err(crate::Error::TempError);
                }
            } else {
                match AshSwapchain::new(
                    self.device.handle.clone(),
                    self.device.swapchain_extension.clone(),
                    self.instance.surface_extension.clone(),
                    self.device.physical_device,
                    surface_swapchains.surface.clone(),
                    swapchain_config,
                ) {
                    Ok(swapchain) => {
                        surface_swapchains
                            .swapchains
                            .insert(self.device.physical_device, swapchain);
                    }
                    Err(_e) => return Err(crate::Error::TempError),
                }
            }
        } else {
            return Err(crate::Error::TempError);
        }

        Ok(())
    }

    fn begin_frame(&self) -> crate::Result<Box<dyn RenderGraphBuilderTrait>> {
        Ok(Box::new(AshRenderGraphBuilder::new(
            self.device.clone(),
            self.surfaces_swapchains.clone(),
        )))
    }

    fn acquire_swapchain_texture(&mut self, surface: SurfaceHandle) -> TransientTexture {
        todo!()
    }

    fn add_transfer_pass(&mut self, name: &str, queue: Queue, transfers: &[Transfer]) {
        todo!()
    }

    fn add_compute_pass(
        &mut self,
        name: &str,
        queue: Queue,
        pipeline: ComputePipeline,
        dispatch_size: &ComputeDispatch,
        resources: &[ShaderResourceAccess],
    ) {
        todo!()
    }

    fn add_raster_pass(
        &mut self,
        name: &str,
        description: &RasterPassDescription,
        raster_commands: &[RasterCommand],
    ) {
        todo!()
    }

    fn submit_frame(&mut self) -> crate::Result<()> {
        todo!()
    }
}
