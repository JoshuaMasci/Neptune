use crate::buffer::{Buffer, BufferDescription, BufferUsage};
use crate::image::{Image, ImageDescription2D};
use crate::instance::AshInstance;
use crate::pipeline::{ComputePipeline, Pipelines, RasterPipeline, RasterPipelineDescription};
use crate::render_graph::CompiledRenderGraph;
use crate::render_graph_builder::{BufferOffset, ImageCopyBuffer, ImageCopyImage};
use crate::render_graph_executor::RenderGraphExecutor;
use crate::resource_managers::ResourceManager;
use crate::sampler::{Sampler, SamplerDescription};
use crate::swapchain::{SurfaceSettings, Swapchain, SwapchainManager};
use crate::upload_queue::UploadQueue;
use crate::{
    BufferHandle, BufferSetHandle, ComputePipelineHandle, ImageHandle, PhysicalDevice,
    RasterPipelineHandle, SamplerHandle, ShaderStage, SurfaceHandle, VulkanError,
};
use ash::vk;
use ash::vk::BufferUsageFlags;
use gpu_allocator::MemoryLocation;
use log::error;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};

#[derive(Copy, Clone, Debug)]
pub struct AshQueue {
    pub family_index: u32,
    pub handle: vk::Queue,
    pub flags: vk::QueueFlags,
}

pub struct AshRaytracing {
    pub acceleration_structure: ash::extensions::khr::AccelerationStructure,
    pub raytracing_pipeline: ash::extensions::khr::RayTracingPipeline,
}

pub struct AshDevice {
    pub instance: Arc<AshInstance>,
    pub physical: vk::PhysicalDevice,
    pub graphics_queue: Option<AshQueue>,
    pub compute_queue: Option<AshQueue>,
    pub transfer_queue: Option<AshQueue>,
    pub core: ash::Device,
    pub swapchain: ash::extensions::khr::Swapchain,
    pub mesh_shading: Option<ash::extensions::ext::MeshShader>,
    pub raytracing: Option<AshRaytracing>,
    pub allocator: ManuallyDrop<Mutex<gpu_allocator::vulkan::Allocator>>,
}

impl AshDevice {
    pub fn new(
        instance: Arc<AshInstance>,
        physical_device: &PhysicalDevice,
    ) -> Result<Self, VulkanError> {
        let mut queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = Vec::with_capacity(3);

        if let Some(queue_family_index) = physical_device.graphics_queue_family_index {
            queue_create_infos.push(
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(queue_family_index)
                    .queue_priorities(&[1.0])
                    .build(),
            );
        }
        if let Some(queue_family_index) = physical_device.compute_queue_family_index {
            queue_create_infos.push(
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(queue_family_index)
                    .queue_priorities(&[1.0])
                    .build(),
            );
        }
        if let Some(queue_family_index) = physical_device.transfer_queue_family_index {
            queue_create_infos.push(
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(queue_family_index)
                    .queue_priorities(&[1.0])
                    .build(),
            );
        }

        let device_extension_names_raw = vec![ash::extensions::khr::Swapchain::name().as_ptr()];

        let mut vulkan_1_2_features = vk::PhysicalDeviceVulkan12Features::builder()
            .buffer_device_address(true)
            .descriptor_indexing(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_storage_buffer_update_after_bind(true)
            .descriptor_binding_storage_image_update_after_bind(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .descriptor_binding_update_unused_while_pending(true)
            .runtime_descriptor_array(true);

        let mut vulkan_1_3_features = vk::PhysicalDeviceVulkan13Features::builder()
            .synchronization2(true)
            .dynamic_rendering(true);

        let mut physical_device_robustness2_features =
            vk::PhysicalDeviceRobustness2FeaturesEXT::builder().null_descriptor(true);

        let core = unsafe {
            instance.core.create_device(
                physical_device.handle,
                &vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&queue_create_infos)
                    .enabled_extension_names(&device_extension_names_raw)
                    .push_next(&mut vulkan_1_2_features)
                    .push_next(&mut vulkan_1_3_features)
                    .push_next(&mut physical_device_robustness2_features)
                    .build(),
                None,
            )
        }?;

        let swapchain = ash::extensions::khr::Swapchain::new(&instance.core, &core);

        let queue_family_properties = unsafe {
            instance
                .core
                .get_physical_device_queue_family_properties(physical_device.handle)
        };

        let graphics_queue = physical_device
            .graphics_queue_family_index
            .map(|family_index| AshQueue {
                family_index,
                handle: unsafe { core.get_device_queue(family_index, 0) },
                flags: queue_family_properties[family_index as usize].queue_flags,
            });

        let compute_queue = physical_device
            .graphics_queue_family_index
            .map(|family_index| AshQueue {
                family_index,
                handle: unsafe { core.get_device_queue(family_index, 0) },
                flags: queue_family_properties[family_index as usize].queue_flags,
            });

        let transfer_queue = physical_device
            .graphics_queue_family_index
            .map(|family_index| AshQueue {
                family_index,
                handle: unsafe { core.get_device_queue(family_index, 0) },
                flags: queue_family_properties[family_index as usize].queue_flags,
            });

        let allocator = ManuallyDrop::new(Mutex::new(gpu_allocator::vulkan::Allocator::new(
            &gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.core.clone(),
                device: core.clone(),
                physical_device: physical_device.handle,
                debug_settings: gpu_allocator::AllocatorDebugSettings::default(),
                buffer_device_address: true,
            },
        )?));

        Ok(Self {
            instance,
            physical: physical_device.handle,
            graphics_queue,
            compute_queue,
            transfer_queue,
            core,
            swapchain,
            mesh_shading: None,
            raytracing: None,
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

pub struct DeviceSettings {
    pub frames_in_flight: u32,
}

pub struct Device {
    settings: DeviceSettings,
    device: Arc<AshDevice>,
    pipelines: Pipelines,
    resource_manager: ResourceManager,
    swapchain_manager: SwapchainManager,

    upload_queue: UploadQueue,
    graph_executor: RenderGraphExecutor,
}

impl Device {
    pub fn new(
        instance: Arc<AshInstance>,
        physical_device: PhysicalDevice,
        settings: DeviceSettings,
    ) -> Result<Device, VulkanError> {
        let push_constant_size = unsafe {
            instance
                .core
                .get_physical_device_properties(physical_device.handle)
        }
        .limits
        .max_push_constants_size;

        let device = AshDevice::new(instance, &physical_device).map(Arc::new)?;
        let resource_manager = ResourceManager::new(device.clone(), settings.frames_in_flight);
        let swapchain_manager = SwapchainManager::new(device.instance.clone());

        let pipelines = Pipelines::new(device.clone(), unsafe {
            device.core.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&[resource_manager.descriptor_set.get_layout()])
                    .push_constant_ranges(&[vk::PushConstantRange {
                        stage_flags: vk::ShaderStageFlags::ALL,
                        offset: 0,
                        size: push_constant_size,
                    }]),
                None,
            )?
        });

        let upload_queue = UploadQueue::default();
        let graph_executor = RenderGraphExecutor::new(device.clone(), settings.frames_in_flight)?;

        Ok(Device {
            settings,
            device,
            pipelines,
            resource_manager,
            swapchain_manager,
            upload_queue,
            graph_executor,
        })
    }

    pub fn create_buffer(
        &mut self,
        name: &str,
        size: usize,
        usage: BufferUsage,
        location: MemoryLocation,
    ) -> Result<BufferHandle, VulkanError> {
        Ok(BufferHandle::Persistent(
            self.resource_manager
                .create_buffer(name, size, usage, location)?,
        ))
    }

    pub fn destroy_buffer(&mut self, buffer_handle: BufferHandle) {
        match buffer_handle {
            BufferHandle::Persistent(key) => self.resource_manager.remove_buffer(key),
            BufferHandle::Transient(index) => {
                error!("Transient buffer {index} cannot be destroyed, this shouldn't happen")
            }
        }
    }

    pub fn update_data_to_buffer(
        &mut self,
        buffer_handle: BufferHandle,
        buffer_offset: u32,
        data: &[u8],
    ) -> Result<(), VulkanError> {
        let mut staging_buffer = Buffer::new(
            self.device.clone(),
            "Stating Buffer",
            &BufferDescription {
                size: data.len() as vk::DeviceSize,
                usage: vk::BufferUsageFlags::TRANSFER_SRC,
                location: gpu_allocator::MemoryLocation::CpuToGpu,
            },
        )?;

        let mut_slice = match staging_buffer.allocation.mapped_slice_mut() {
            None => return Err(VulkanError::Vk(vk::Result::ERROR_MEMORY_MAP_FAILED)),
            Some(mut_slice) => mut_slice,
        };
        mut_slice[0..data.len()].copy_from_slice(data);

        let staging_handle =
            BufferHandle::Persistent(self.resource_manager.add_buffer(staging_buffer));

        self.upload_queue.add_buffer_upload(
            BufferOffset {
                buffer: staging_handle,
                offset: 0,
            },
            BufferOffset {
                buffer: buffer_handle,
                offset: buffer_offset as usize,
            },
            data.len(),
        );

        //Destroy stating buffer once frame is done
        self.destroy_buffer(staging_handle);

        Ok(())
    }
    pub fn create_buffer_init(
        &mut self,
        name: &str,
        usage: BufferUsage,
        location: gpu_allocator::MemoryLocation,
        data: &[u8],
    ) -> Result<BufferHandle, VulkanError> {
        let buffer = self.create_buffer(name, data.len(), usage, location)?;
        self.update_data_to_buffer(buffer, 0, data)?;
        Ok(buffer)
    }

    pub fn create_image(
        &mut self,
        name: &str,
        description: &ImageDescription2D,
    ) -> Result<ImageHandle, VulkanError> {
        let image = Image::new_2d(self.device.clone(), name, description)?;

        Ok(ImageHandle::Persistent(
            self.resource_manager.add_image(image),
        ))
    }
    pub fn destroy_image(&mut self, image_handle: ImageHandle) {
        match image_handle {
            ImageHandle::Persistent(key) => self.resource_manager.remove_image(key),
            ImageHandle::Transient(index) => {
                error!("Transient image {index} cannot be destroyed, this shouldn't happen")
            }
        }
    }
    pub fn update_data_to_image(
        &mut self,
        image_handle: ImageHandle,
        image_size: [u32; 2],
        data: &[u8],
    ) -> Result<(), VulkanError> {
        let mut staging_buffer = Buffer::new(
            self.device.clone(),
            "Stating Buffer",
            &BufferDescription {
                size: data.len() as vk::DeviceSize,
                usage: vk::BufferUsageFlags::TRANSFER_SRC,
                location: gpu_allocator::MemoryLocation::CpuToGpu,
            },
        )?;

        let mut_slice = match staging_buffer.allocation.mapped_slice_mut() {
            None => return Err(VulkanError::Vk(vk::Result::ERROR_MEMORY_MAP_FAILED)),
            Some(mut_slice) => mut_slice,
        };
        mut_slice.copy_from_slice(data);

        let staging_handle =
            BufferHandle::Persistent(self.resource_manager.add_buffer(staging_buffer));

        self.upload_queue.add_image_upload(
            ImageCopyBuffer {
                buffer: staging_handle,
                offset: 0,
                row_length: None,
                row_height: None,
            },
            ImageCopyImage {
                image: image_handle,
                offset: [0, 0],
            },
            image_size,
        );

        //Destroy stating buffer once frame is done
        self.destroy_buffer(staging_handle);

        Ok(())
    }
    pub fn create_image_init(
        &mut self,
        name: &str,
        description: &ImageDescription2D,
        data: &[u8],
    ) -> Result<ImageHandle, VulkanError> {
        let image = self.create_image(name, description)?;
        self.update_data_to_image(image, description.size, data)?;
        Ok(image)
    }

    pub fn create_sampler(
        &mut self,
        name: &str,
        sampler_description: &SamplerDescription,
    ) -> Result<SamplerHandle, VulkanError> {
        Ok(SamplerHandle(self.resource_manager.add_sampler(
            Sampler::new(self.device.clone(), name, sampler_description)?,
        )))
    }
    pub fn destroy_sampler(&mut self, sampler_handle: SamplerHandle) {
        self.resource_manager.remove_sampler(sampler_handle.0);
    }

    // pub fn create_buffer_set(
    //     &mut self,
    //     name: &str,
    //     count: usize,
    // ) -> Result<BufferSetHandle, VulkanError> {
    //     let gpu_buffer_size = std::mem::size_of::<u32>() * count;
    //     let gpu_buffers: Vec<Buffer> = (0..self.settings.frames_in_flight)
    //         .map(|i| {
    //             Buffer::new2(
    //                 self.device.clone(),
    //                 &format!("{} - {}", name, i),
    //                 gpu_buffer_size as vk::DeviceSize,
    //                 BufferUsageFlags::UNIFORM_BUFFER | BufferUsageFlags::TRANSFER_DST,
    //                 MemoryLocation::GpuOnly,
    //             )
    //         })
    //         .collect::<Result<Vec<Buffer>, VulkanError>>()?;
    //
    //     let set = crate::resource_set::BufferSet {
    //         name: name.to_string(),
    //         handles: vec![None; count],
    //         gpu_buffer_index: 0,
    //         gpu_buffers,
    //     };
    //     Ok(BufferSetHandle(self.resource_manager.add_buffer_set(set)))
    // }
    // pub fn destroy_buffer_set(&mut self, buffer_set_handle: BufferSetHandle) {}
    // pub fn update_buffer_set(
    //     &mut self,
    //     buffer_set_handle: BufferSetHandle,
    //     index: usize,
    //     buffer_handle: BufferHandle,
    // ) {
    // }
    // pub fn clear_buffer_set(&mut self, buffer_set_handle: BufferSetHandle, index: usize) {}

    //TODO: use vulkan future and some async pipeline creation method to avoid pipeline creation in the main code paths
    pub fn create_compute_pipeline(
        &mut self,
        shader: &ShaderStage,
    ) -> Result<ComputePipelineHandle, VulkanError> {
        Ok(ComputePipelineHandle(self.pipelines.compute.insert(
            ComputePipeline::new(self.device.clone(), self.pipelines.layout, shader)?,
        )))
    }
    pub fn destroy_compute_pipeline(&mut self, compute_pipeline_handle: ComputePipelineHandle) {
        drop(self.pipelines.compute.remove(compute_pipeline_handle.0))
    }

    //TODO: allow multiple creation of multiple pipelines at once?
    //TODO: use vulkan future and some async pipeline creation method to avoid pipeline creation in the main code paths
    pub fn create_raster_pipeline(
        &mut self,
        description: &RasterPipelineDescription,
    ) -> Result<RasterPipelineHandle, VulkanError> {
        Ok(RasterPipelineHandle(self.pipelines.raster.insert(
            RasterPipeline::new(self.device.clone(), self.pipelines.layout, description)?,
        )))
    }
    pub fn destroy_raster_pipeline(&mut self, raster_pipeline_handle: RasterPipelineHandle) {
        drop(self.pipelines.raster.remove(raster_pipeline_handle.0))
    }

    pub fn configure_surface(
        &mut self,
        surface_handle: SurfaceHandle,
        settings: &SurfaceSettings,
    ) -> Result<(), VulkanError> {
        if let Some(swapchain) = self.swapchain_manager.get(surface_handle) {
            swapchain.update_settings(settings)?;
        } else {
            self.swapchain_manager.add(Swapchain::new(
                self.device.clone(),
                surface_handle,
                settings,
            )?);
        }

        Ok(())
    }
    pub fn release_surface(&mut self, surface_handle: SurfaceHandle) {
        self.swapchain_manager.remove(surface_handle);
    }

    pub fn submit_graph(&mut self, render_graph: &CompiledRenderGraph) -> Result<(), VulkanError> {
        self.graph_executor.submit_frame(
            &mut self.resource_manager,
            &mut self.swapchain_manager,
            &self.pipelines,
            self.upload_queue.get_pass(),
            render_graph,
        )?;
        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.core.device_wait_idle();
        }
    }
}
