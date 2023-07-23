use crate::render_graph::RenderGraph;
use crate::traits::DeviceTrait;
use crate::vulkan::buffer::AshBuffer;
use crate::vulkan::image::AshImage;
use crate::vulkan::instance::{AshInstance, AshPhysicalDeviceQueues};
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
    TextureHandle, TextureUsage, Transfer, TransientBuffer, TransientTexture,
};
use ash::vk;
use gpu_allocator::MemoryLocation;
use slotmap::{KeyData, SlotMap};
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum VulkanCreateError {
    #[error("Vk error: {0}")]
    VkError(vk::Result),

    #[error("Gpu alloc error: {0}")]
    GpuAllocError(gpu_allocator::AllocationError),
}

impl From<vk::Result> for VulkanCreateError {
    fn from(value: vk::Result) -> Self {
        VulkanCreateError::VkError(value)
    }
}

impl From<gpu_allocator::AllocationError> for VulkanCreateError {
    fn from(value: gpu_allocator::AllocationError) -> Self {
        VulkanCreateError::GpuAllocError(value)
    }
}

#[derive(Clone)]
pub(crate) struct AshQueue {
    pub(crate) handle: vk::Queue,
    pub(crate) family_index: u32,
}

pub(crate) struct AshCommandPool {
    device: Arc<AshDevice>,
    pub(crate) handle: vk::CommandPool,
    pub(crate) freed_buffers: Vec<vk::CommandBuffer>,
}

impl AshCommandPool {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        family_index: u32,
        flags: vk::CommandPoolCreateFlags,
    ) -> Result<Self, VulkanCreateError> {
        let handle = unsafe {
            device.core.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(family_index)
                    .flags(flags),
                None,
            )
        }?;

        Ok(Self {
            device,
            handle,
            freed_buffers: Vec::new(),
        })
    }

    pub(crate) fn get(&mut self) -> vk::CommandBuffer {
        if let Some(command_buffer) = self.freed_buffers.pop() {
            command_buffer
        } else {
            unsafe {
                self.device
                    .core
                    .allocate_command_buffers(
                        &vk::CommandBufferAllocateInfo::builder()
                            .command_pool(self.handle)
                            .command_buffer_count(1),
                    )
                    .unwrap()[0]
            }
        }
    }

    pub(crate) fn free(&mut self, command_buffer: vk::CommandBuffer) {
        self.freed_buffers.push(command_buffer)
    }
}

impl Drop for AshCommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.core.destroy_command_pool(self.handle, None);
        }
    }
}

pub(crate) struct AshRaytracingExtension {
    pub(crate) acceleration_structure: ash::extensions::khr::AccelerationStructure,
    pub(crate) ray_tracing_pipeline: ash::extensions::khr::RayTracingPipeline,
}

pub(crate) struct AshDevice {
    pub(crate) instance: Arc<AshInstance>,
    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) core: ash::Device,
    pub(crate) swapchain: ash::extensions::khr::Swapchain,
    pub(crate) mesh_shading: Option<ash::extensions::ext::MeshShader>,
    pub(crate) ray_tracing: Option<AshRaytracingExtension>,

    pub(crate) graphics_queue: AshQueue,
    pub(crate) compute_queue: Option<AshQueue>,
    pub(crate) transfer_queue: Option<AshQueue>,

    pub allocator: ManuallyDrop<Mutex<gpu_allocator::vulkan::Allocator>>,
}

impl AshDevice {
    fn new(
        instance: Arc<AshInstance>,
        physical_device: vk::PhysicalDevice,
        queues: &AshPhysicalDeviceQueues,
        extensions: &PhysicalDeviceExtensions,
    ) -> Result<Self, VulkanCreateError> {
        let mut device_extension_names_raw = vec![ash::extensions::khr::Swapchain::name().as_ptr()];

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

        let mut robustness2_features =
            vk::PhysicalDeviceRobustness2FeaturesEXT::builder().null_descriptor(true);

        let mut vulkan12_features = vk::PhysicalDeviceVulkan12Features::builder()
            .descriptor_indexing(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_uniform_buffer_update_after_bind(true)
            .descriptor_binding_storage_buffer_update_after_bind(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .descriptor_binding_storage_image_update_after_bind(true)
            .descriptor_binding_update_unused_while_pending(true)
            .buffer_device_address(true);

        let mut vulkan13_features = vk::PhysicalDeviceVulkan13Features::builder()
            .synchronization2(true)
            .dynamic_rendering(true);

        let priorities = &[1.0];
        let mut queue_info = vec![vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queues.graphics_queue_family_index)
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

        let core = unsafe {
            instance.core.create_device(
                physical_device,
                &vk::DeviceCreateInfo::builder()
                    .push_next(&mut robustness2_features)
                    .push_next(&mut vulkan12_features)
                    .push_next(&mut vulkan13_features)
                    .queue_create_infos(&queue_info)
                    .enabled_extension_names(&device_extension_names_raw),
                None,
            )
        }?;

        let swapchain = ash::extensions::khr::Swapchain::new(&instance.core, &core);

        let mesh_shading = extensions
            .mesh_shading
            .then(|| ash::extensions::ext::MeshShader::new(&instance.core, &core));

        let ray_tracing = extensions.ray_tracing.then(|| AshRaytracingExtension {
            acceleration_structure: ash::extensions::khr::AccelerationStructure::new(
                &instance.core,
                &core,
            ),
            ray_tracing_pipeline: ash::extensions::khr::RayTracingPipeline::new(
                &instance.core,
                &core,
            ),
        });

        let graphics_queue = AshQueue {
            handle: unsafe { core.get_device_queue(queues.graphics_queue_family_index, 0) },
            family_index: queues.graphics_queue_family_index,
        };

        let compute_queue = queues
            .compute_queue_family_index
            .map(|family_index| AshQueue {
                handle: unsafe { core.get_device_queue(family_index, 0) },
                family_index,
            });

        let transfer_queue: Option<AshQueue> =
            queues
                .transfer_queue_family_index
                .map(|family_index| AshQueue {
                    handle: unsafe { core.get_device_queue(family_index, 0) },
                    family_index,
                });

        //TODO: return error
        let allocator =
            gpu_allocator::vulkan::Allocator::new(&gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.core.clone(),
                device: core.clone(),
                physical_device,
                debug_settings: gpu_allocator::AllocatorDebugSettings::default(),
                buffer_device_address: true,
            })?;
        let allocator = ManuallyDrop::new(Mutex::new(allocator));

        Ok(Self {
            instance,
            physical_device,
            core,
            swapchain,
            mesh_shading,
            ray_tracing,
            graphics_queue,
            compute_queue,
            transfer_queue,
            allocator,
        })
    }
}

impl Drop for AshDevice {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.allocator);
            self.core.destroy_device(None);
        }
    }
}

pub(crate) struct DeviceFrame {
    device: Arc<AshDevice>,
    graphics_command_pool: AshCommandPool,
    compute_command_pool: Option<AshCommandPool>,
    transfer_command_pool: Option<AshCommandPool>,

    //TEMP
    frame_done_fence: vk::Fence,
    image_ready_semaphore: vk::Semaphore,
    frame_done_semaphore: vk::Semaphore,
}

impl DeviceFrame {
    fn new(device: Arc<AshDevice>) -> Result<Self, VulkanCreateError> {
        let pool_flags = vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER;

        let graphics_command_pool = AshCommandPool::new(
            device.clone(),
            device.graphics_queue.family_index,
            pool_flags,
        )?;

        let mut compute_command_pool = None;
        if let Some(compute_queue) = &device.compute_queue {
            compute_command_pool = Some(AshCommandPool::new(
                device.clone(),
                compute_queue.family_index,
                pool_flags,
            )?);
        }

        let mut transfer_command_pool = None;
        if let Some(transfer_queue) = &device.transfer_queue {
            transfer_command_pool = Some(AshCommandPool::new(
                device.clone(),
                transfer_queue.family_index,
                pool_flags,
            )?);
        }

        unsafe {
            let pool_flags = vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER;
            Ok(Self {
                graphics_command_pool,
                compute_command_pool,
                transfer_command_pool,
                frame_done_fence: device.core.create_fence(
                    &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )?,
                image_ready_semaphore: device
                    .core
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?,
                frame_done_semaphore: device
                    .core
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?,
                device,
            })
        }
    }
}

impl Drop for DeviceFrame {
    fn drop(&mut self) {
        unsafe {
            self.device.core.destroy_fence(self.frame_done_fence, None);
            self.device
                .core
                .destroy_semaphore(self.image_ready_semaphore, None);
            self.device
                .core
                .destroy_semaphore(self.frame_done_semaphore, None);
        }
    }
}

pub struct Device {
    device: Arc<AshDevice>,
    buffers: SlotMap<AshBufferHandle, Arc<AshBuffer>>,
    textures: SlotMap<AshTextureHandle, Arc<AshImage>>,
    samplers: SlotMap<AshSamplerHandle, Arc<AshSampler>>,
    swapchains: HashMap<AshSurfaceHandle, AshSwapchain>,

    frames: Vec<DeviceFrame>,
}

impl Device {
    pub(crate) fn new(
        instance: &Instance,
        device_index: usize,
        create_info: &DeviceCreateInfo,
    ) -> Result<Self, VulkanCreateError> {
        let physical_device = &instance.physical_devices[device_index];

        //TODO: verify that queue and extensions are supported

        let mut queues = physical_device.queues.clone();

        if !create_info.features.async_compute {
            queues.compute_queue_family_index = None;
        }

        if !create_info.features.async_transfer {
            queues.transfer_queue_family_index = None;
        }

        let device = Arc::new(AshDevice::new(
            instance.instance.clone(),
            physical_device.handle,
            &queues,
            &create_info.extensions,
        )?);

        let frames = (0..create_info.frames_in_flight_count)
            .map(|_| DeviceFrame::new(device.clone()).unwrap())
            .collect();

        Ok(Self {
            device,
            buffers: SlotMap::with_key(),
            textures: SlotMap::with_key(),
            samplers: SlotMap::with_key(),
            swapchains: HashMap::new(),
            frames,
        })
    }
}

impl DeviceTrait for Device {
    fn create_buffer(
        &mut self,
        name: &str,
        description: &BufferDescription,
    ) -> crate::Result<BufferHandle> {
        let create_info = vk::BufferCreateInfo::builder()
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .usage(description.usage.to_vk())
            .size(description.size)
            .build();

        let buffer = match AshBuffer::new(
            self.device.clone(),
            name,
            &create_info,
            MemoryLocation::GpuOnly,
        ) {
            Ok(buffer) => buffer,
            Err(_e) => return Err(crate::Error::TempError),
        };

        if let Some(debug_utils) = &self.device.instance.debug_utils {
            let _ = debug_utils.set_object_name(self.device.core.handle(), buffer.handle, name);
        }

        let buffer_handle = self.buffers.insert(Arc::new(buffer));
        Ok(buffer_handle.0.as_ffi())
    }

    fn destroy_buffer(&mut self, handle: BufferHandle) {
        let buffer_handle = AshBufferHandle::from(KeyData::from_ffi(handle));
        let _ = self.buffers.remove(buffer_handle);
    }

    fn create_texture(
        &mut self,
        name: &str,
        description: &TextureDescription<[u32; 2]>,
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
            self.device.clone(),
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

        if let Some(debug_utils) = &self.device.instance.debug_utils {
            let _ = debug_utils.set_object_name(self.device.core.handle(), image.handle, name);
            let _ = debug_utils.set_object_name(self.device.core.handle(), image.view_handle, name);
        }

        let texture_handle = self.textures.insert(Arc::new(image));
        Ok(texture_handle.0.as_ffi())
    }

    fn destroy_texture(&mut self, handle: TextureHandle) {
        let texture_handle = AshTextureHandle::from(KeyData::from_ffi(handle));
        let _ = self.textures.remove(texture_handle);
    }

    fn create_sampler(
        &mut self,
        name: &str,
        description: &SamplerDescription,
    ) -> crate::Result<SamplerHandle> {
        let sampler = match AshSampler::new(self.device.clone(), &description.to_vk()) {
            Ok(sampler) => sampler,
            Err(_e) => return Err(crate::Error::TempError),
        };

        if let Some(debug_utils) = &self.device.instance.debug_utils {
            let _ = debug_utils.set_object_name(self.device.core.handle(), sampler.handle, name);
        }

        let sampler_handle = self.samplers.insert(Arc::new(sampler));
        Ok(sampler_handle.0.as_ffi())
    }

    fn destroy_sampler(&mut self, handle: SamplerHandle) {
        let sampler_handle = AshSamplerHandle::from(KeyData::from_ffi(handle));
        let _ = self.samplers.remove(sampler_handle);
    }

    fn create_compute_pipeline(
        &mut self,
        name: &str,
        description: &ComputePipelineDescription,
    ) -> crate::Result<ComputePipelineHandle> {
        todo!()
    }

    fn destroy_compute_pipeline(&mut self, handle: ComputePipelineHandle) {
        todo!()
    }

    fn create_raster_pipeline(
        &mut self,
        name: &str,
        description: &RasterPipelineDescription,
    ) -> crate::Result<RasterPipelineHandle> {
        todo!()
    }

    fn destroy_raster_pipeline(&mut self, handle: RasterPipelineHandle) {
        todo!()
    }

    fn configure_surface(
        &mut self,
        surface_handle: SurfaceHandle,
        description: &SwapchainDescription,
    ) -> crate::Result<()> {
        let surface_handle = AshSurfaceHandle::from(KeyData::from_ffi(surface_handle));

        let swapchain_config = SwapchainConfig {
            image_count: description.min_image_count,
            format: vk::SurfaceFormatKHR {
                format: description.surface_format.format.to_vk(),
                color_space: description.surface_format.color_space.to_vk(),
            },
            present_mode: description.present_mode.to_vk(),
            usage: description.usage.to_vk(true),
            composite_alpha: description.composite_alpha.to_vk(),
        };

        if let Some(surface) = self
            .device
            .instance
            .surfaces
            .lock()
            .unwrap()
            .get(surface_handle)
        {
            if let Some(swapchain) = self.swapchains.get_mut(&surface_handle) {
                if let Err(_e) = swapchain.update_config(swapchain_config) {
                    return Err(crate::Error::TempError);
                }
            } else {
                match AshSwapchain::new(self.device.clone(), surface.clone(), swapchain_config) {
                    Ok(swapchain) => {
                        let _ = self.swapchains.insert(surface_handle, swapchain);
                    }
                    Err(_e) => return Err(crate::Error::TempError),
                };
            }

            Ok(())
        } else {
            Err(crate::Error::TempError)
        }
    }

    fn release_surface(&mut self, surface_handle: SurfaceHandle) {
        let surface_handle = AshSurfaceHandle::from(KeyData::from_ffi(surface_handle));
        let _ = self.swapchains.remove(&surface_handle);
    }

    fn submit_frame(&mut self, render_graph: &RenderGraph) -> crate::Result<()> {
        let _ = render_graph;

        let swapchain = render_graph
            .swapchain_usage
            .first()
            .and_then(|surface_handle| {
                self.swapchains
                    .get_mut(&AshSurfaceHandle::from(KeyData::from_ffi(*surface_handle)))
            });

        if let Some(swapchain) = swapchain {
            let frame = &self.frames[0];
        }

        // let frame = self.frames[0].lock().unwrap();
        // let mut lock = self.temp_used_surfaces.lock().unwrap();
        // lock.clear();
        //
        // unsafe {
        //     let frame_done_fence = self
        //         .device
        //         .core
        //         .create_fence(&vk::FenceCreateInfo::default(), None)
        //         .unwrap();
        //
        //     let command_buffer = self
        //         .device
        //         .core
        //         .allocate_command_buffers(
        //             &vk::CommandBufferAllocateInfo::builder()
        //                 .command_pool(frame.primary_command_pool)
        //                 .command_buffer_count(1),
        //         )
        //         .unwrap()[0];
        //
        //     self.device
        //         .core
        //         .begin_command_buffer(command_buffer, &vk::CommandBufferBeginInfo::default())
        //         .unwrap();
        //     self.device.core.end_command_buffer(command_buffer).unwrap();
        //
        //     self.device
        //         .core
        //         .queue_submit(
        //             self.device.graphics_queue.handle,
        //             &[vk::SubmitInfo::builder()
        //                 .command_buffers(&[command_buffer])
        //                 .build()],
        //             frame_done_fence,
        //         )
        //         .unwrap();
        //
        //     self.device
        //         .core
        //         .wait_for_fences(
        //             &[frame_done_fence],
        //             true,
        //             std::time::Duration::from_millis(2).as_nanos() as u64,
        //         )
        //         .unwrap();
        //
        //     self.device.core.destroy_fence(frame_done_fence, None);
        //     self.device
        //         .core
        //         .free_command_buffers(frame.primary_command_pool, &[command_buffer]);
        // }
        //

        Ok(())
    }
}
