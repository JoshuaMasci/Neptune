use crate::render_graph::{BasicLinearRenderGraphExecutor, RenderGraphBuilder};
use crate::resource_manager::{
    BufferHandle, ComputePipelineHandle, ResourceManager, SamplerHandle, TextureHandle,
};
use crate::sampler::SamplerCreateInfo;
use crate::surface::Surface;
use crate::swapchain::{AshSwapchain, SwapchainConfig};
use crate::{AshInstance, BufferUsage};
use crate::{Error, GpuResource, SwapchainHandle};
use crate::{GpuResourcePool, TextureUsage};
use ash::vk;
use std::ffi::CStr;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

pub struct Swapchain(pub(crate) GpuResource<SwapchainHandle, Arc<Mutex<AshSwapchain>>>);

impl Swapchain {
    pub fn update(&self, swapchain_config: SwapchainConfig) -> crate::Result<()> {
        match self.0.pool.lock() {
            Ok(pool_lock) => match pool_lock.get(self.0.handle) {
                Some(swapchain) => match swapchain.lock() {
                    Ok(mut swapchain_lock) => swapchain_lock.update(swapchain_config),
                    Err(_) => Err(crate::Error::StringError(String::from("Mutex Lock Error"))),
                },
                None => Err(crate::Error::StringError(String::from(
                    "Swapchain Handle not valid",
                ))),
            },
            Err(_) => Err(crate::Error::StringError(String::from("Mutex Lock Error"))),
        }
    }
}

pub struct Buffer {
    pub(crate) handle: BufferHandle,
    resource_manager: Arc<Mutex<ResourceManager>>,
}
impl Drop for Buffer {
    fn drop(&mut self) {
        self.resource_manager
            .lock()
            .unwrap()
            .destroy_buffer(self.handle);
    }
}

pub struct Texture {
    pub(crate) handle: TextureHandle,
    resource_manager: Arc<Mutex<ResourceManager>>,
}
impl Drop for Texture {
    fn drop(&mut self) {
        self.resource_manager
            .lock()
            .unwrap()
            .destroy_texture(self.handle);
    }
}

pub struct Sampler {
    pub(crate) handle: SamplerHandle,
    resource_manager: Arc<Mutex<ResourceManager>>,
}
impl Drop for Sampler {
    fn drop(&mut self) {
        self.resource_manager
            .lock()
            .unwrap()
            .destroy_sampler(self.handle);
    }
}

pub struct ComputePipeline {
    pub(crate) handle: ComputePipelineHandle,
    resource_manager: Arc<Mutex<ResourceManager>>,
}
impl Drop for ComputePipeline {
    fn drop(&mut self) {
        self.resource_manager
            .lock()
            .unwrap()
            .destroy_compute_pipeline(self.handle);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Integrated,
    Discrete,
    Unknown,
}

impl DeviceType {
    fn from_vk(device_type: vk::PhysicalDeviceType) -> Self {
        match device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => Self::Discrete,
            vk::PhysicalDeviceType::INTEGRATED_GPU => Self::Integrated,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceVendor {
    Amd,
    Arm,
    ImgTec,
    Intel,
    Nvidia,
    Qualcomm,
    Unknown(u32),
}

impl DeviceVendor {
    fn from_vk(vendor_id: u32) -> Self {
        //TODO: find a place to verify this list
        match vendor_id {
            0x1002 => DeviceVendor::Amd,
            0x10DE => DeviceVendor::Nvidia,
            0x8086 => DeviceVendor::Intel,
            0x1010 => DeviceVendor::ImgTec,
            0x13B5 => DeviceVendor::Arm,
            0x5132 => DeviceVendor::Qualcomm,
            x => DeviceVendor::Unknown(x),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub vendor: DeviceVendor,
    pub device_type: DeviceType,
}

impl DeviceInfo {
    pub(crate) fn new(physical_device_properties: vk::PhysicalDeviceProperties) -> Self {
        Self {
            name: String::from(
                unsafe { CStr::from_ptr(physical_device_properties.device_name.as_ptr()) }
                    .to_str()
                    .expect("Failed to convert CStr to string"),
            ),
            vendor: DeviceVendor::from_vk(physical_device_properties.vendor_id),
            device_type: DeviceType::from_vk(physical_device_properties.device_type),
        }
    }
}

#[derive(Clone)]
pub struct AshDevice(ash::Device);

impl AshDevice {
    fn new(device: ash::Device) -> Self {
        Self(device)
    }
}

impl Deref for AshDevice {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for AshDevice {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_device(None);
            trace!("Drop Device");
        }
    }
}

pub struct Device {
    swapchains: GpuResourcePool<SwapchainHandle, Arc<Mutex<AshSwapchain>>>,
    resource_manager: Arc<Mutex<ResourceManager>>,
    render_graph_executor: Mutex<BasicLinearRenderGraphExecutor>,
    swapchain_ext: Arc<ash::extensions::khr::Swapchain>,
    device: Arc<AshDevice>,
    instance: Arc<AshInstance>,

    info: DeviceInfo,
    physical_device: vk::PhysicalDevice,

    graphics_queue: vk::Queue,
}

impl Device {
    pub(crate) fn new(instance: Arc<AshInstance>, device_index: usize) -> crate::Result<Self> {
        //TODO: check for extension support
        let device_extension_names_raw = vec![ash::extensions::khr::Swapchain::name().as_ptr()];

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

        let physical_device = instance.physical_devices[device_index].clone();

        let priorities = &[1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(physical_device.graphics_queue_family_index)
            .queue_priorities(priorities)
            .build()];

        let device = match unsafe {
            instance.instance.create_device(
                physical_device.handle,
                &vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&queue_info)
                    .enabled_extension_names(&device_extension_names_raw)
                    .push_next(&mut synchronization2_features)
                    .push_next(&mut robustness2_features)
                    .push_next(&mut vulkan1_2_features),
                None,
            )
        } {
            Ok(device) => device,
            Err(e) => return Err(Error::VkError(e)),
        };

        let swapchain_ext = Arc::new(ash::extensions::khr::Swapchain::new(
            &instance.instance,
            &device,
        ));

        let graphics_queue =
            unsafe { device.get_device_queue(physical_device.graphics_queue_family_index, 0) };

        let device = Arc::new(AshDevice::new(device));

        let allocator = match gpu_allocator::vulkan::Allocator::new(
            &gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.instance.clone(),
                device: (**device).clone(),
                physical_device: physical_device.handle,
                debug_settings: gpu_allocator::AllocatorDebugSettings::default(),
                buffer_device_address: false,
            },
        ) {
            Ok(allocator) => Arc::new(Mutex::new(allocator)),
            Err(e) => return Err(Error::GpuAllocError(e)),
        };

        const FRAMES_IN_FLIGHT_COUNT: usize = 3;
        let resource_manager = Arc::new(Mutex::new(ResourceManager::new(
            FRAMES_IN_FLIGHT_COUNT,
            device.clone(),
            allocator,
            instance.debug_utils.clone(),
        )?));

        let render_graph_executor = Mutex::new(BasicLinearRenderGraphExecutor::new(
            device.clone(),
            swapchain_ext.clone(),
            (graphics_queue, physical_device.graphics_queue_family_index),
        ));

        let swapchains = GpuResourcePool::new();

        Ok(Self {
            swapchains,
            resource_manager,
            render_graph_executor,
            swapchain_ext,
            device,
            instance,
            physical_device: physical_device.handle,
            info: physical_device.device_info,
            graphics_queue,
        })
    }

    pub fn info(&self) -> DeviceInfo {
        self.info.clone()
    }

    //TODO: Need a function to query swapchain details
    // pub fn query_swapchain_support(&self, surface: &Surface) -> crate::Result<SwapchainConfig>

    pub fn create_swapchain(
        &self,
        surface: &Surface,
        swapchain_config: SwapchainConfig,
    ) -> crate::Result<Swapchain> {
        let surface = surface
            .0
            .pool
            .lock()
            .unwrap()
            .get(surface.0.handle)
            .unwrap()
            .get_handle();

        let swapchain = Arc::new(Mutex::new(AshSwapchain::new(
            self.physical_device,
            self.device.clone(),
            surface,
            self.instance.surface_ext.clone(),
            self.swapchain_ext.clone(),
            swapchain_config,
        )?));

        Ok(Swapchain(GpuResource::new(
            self.swapchains.lock().unwrap().insert(swapchain),
            self.swapchains.clone(),
        )))
    }

    pub fn create_buffer(
        &self,
        name: &str,
        usage: BufferUsage,
        size: u64,
    ) -> crate::Result<Buffer> {
        self.resource_manager
            .lock()
            .unwrap()
            .create_buffer(name, usage, size)
            .map(|handle| Buffer {
                handle,
                resource_manager: self.resource_manager.clone(),
            })
    }

    pub fn create_buffer_with_data(
        &self,
        name: &str,
        usage: BufferUsage,
        data: &[u8],
    ) -> crate::Result<Buffer> {
        self.resource_manager
            .lock()
            .unwrap()
            .create_buffer(name, usage, data.len() as u64)
            .map(|handle| Buffer {
                handle,
                resource_manager: self.resource_manager.clone(),
            })
    }

    pub fn create_texture(
        &self,
        name: &str,
        usage: TextureUsage,
        format: vk::Format,
        size: [u32; 2],
        sampler: Option<&Sampler>,
    ) -> crate::Result<Texture> {
        self.resource_manager
            .lock()
            .unwrap()
            .create_texture(name, usage, format, size, sampler)
            .map(|handle| Texture {
                handle,
                resource_manager: self.resource_manager.clone(),
            })
    }

    pub fn create_texture_with_data(
        &self,
        name: &str,
        usage: TextureUsage,
        format: vk::Format,
        size: [u32; 2],
        sampler: Option<&Sampler>,
        data: &[u8],
    ) -> crate::Result<Texture> {
        self.resource_manager
            .lock()
            .unwrap()
            .create_texture(name, usage, format, size, sampler)
            .map(|handle| Texture {
                handle,
                resource_manager: self.resource_manager.clone(),
            })
    }

    pub fn create_sampler(
        &self,
        name: &str,
        sampler_create_info: &SamplerCreateInfo,
    ) -> crate::Result<Sampler> {
        self.resource_manager
            .lock()
            .unwrap()
            .create_sampler(name, sampler_create_info)
            .map(|handle| Sampler {
                handle,
                resource_manager: self.resource_manager.clone(),
            })
    }

    pub fn create_compute_pipeline(
        &self,
        name: &str,
        code: &[u32],
    ) -> crate::Result<ComputePipeline> {
        self.resource_manager
            .lock()
            .unwrap()
            .create_compute_pipeline(name, code)
            .map(|handle| ComputePipeline {
                handle,
                resource_manager: self.resource_manager.clone(),
            })
    }

    //TODO: use capture since render_graph_builder will need access to the transfer queue
    pub fn render_frame(&self, render_fn: impl FnOnce(&mut RenderGraphBuilder)) {
        self.resource_manager.lock().unwrap().update();

        let mut render_graph_builder = RenderGraphBuilder::new();

        render_fn(&mut render_graph_builder);

        self.render_graph_executor
            .lock()
            .unwrap()
            .execute_graph(render_graph_builder);

        for (_, swapchain) in self.swapchains.lock().unwrap().iter_mut() {
            let image_ready_semaphore = unsafe {
                self.device
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                    .unwrap()
            };
            let mut lock = swapchain.lock().unwrap();
            match lock.acquire_next_image(image_ready_semaphore) {
                Ok(result) => {
                    unsafe {
                        let _ = self.swapchain_ext.queue_present(
                            self.graphics_queue,
                            &vk::PresentInfoKHR::builder()
                                .swapchains(&[lock.get_handle()])
                                .wait_semaphores(&[image_ready_semaphore])
                                .image_indices(&[result.index]),
                        );
                    }
                    if result.suboptimal {
                        lock.rebuild().unwrap();
                    }
                }
                Err(_) => {
                    lock.rebuild().unwrap();
                }
            }
            unsafe {
                self.device.destroy_semaphore(image_ready_semaphore, None);
            }
        }
    }
}
