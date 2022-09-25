use crate::buffer::Buffer;
use crate::device::DeviceInfo;
use crate::render_graph::RenderGraphBuilder;
use crate::sampler::Sampler;
use crate::shader::{ComputeShader, FragmentShader, VertexShader};
use crate::texture::{SwapchainTexture, Texture};
use crate::vulkan::instance::PhysicalDevice;
use crate::{BufferUsage, DeviceTrait, SamplerCreateInfo, TextureCreateInfo};
use ash::vk;
use std::cell::RefCell;
use std::rc::Rc;

struct DeviceDrop(Rc<ash::Device>);
impl DeviceDrop {
    fn new(device: &Rc<ash::Device>) -> Self {
        Self(device.clone())
    }
}

impl Drop for DeviceDrop {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_device(None);
        }
    }
}

pub struct VulkanDevice {
    info: DeviceInfo,

    physical_device: vk::PhysicalDevice,
    device: Rc<ash::Device>,

    allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,
    device_drop: DeviceDrop,

    graphics_queue: vk::Queue,
}

impl VulkanDevice {
    pub(crate) fn new(instance: &ash::Instance, physical_device: &PhysicalDevice) -> Self {
        let device_extension_names_raw = vec![ash::extensions::khr::Swapchain::name().as_ptr()];

        let mut synchronization2_features =
            vk::PhysicalDeviceSynchronization2FeaturesKHR::builder()
                .synchronization2(true)
                .build();

        let priorities = &[1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(physical_device.graphics_queue_family_index)
            .queue_priorities(priorities)
            .build()];

        //Load all device functions
        let device = Rc::new(
            unsafe {
                instance.create_device(
                    physical_device.handle,
                    &vk::DeviceCreateInfo::builder()
                        .queue_create_infos(&queue_info)
                        .enabled_extension_names(&device_extension_names_raw)
                        .push_next(&mut synchronization2_features),
                    None,
                )
            }
            .expect("Failed to initialize vulkan device"),
        );

        let graphics_queue =
            unsafe { device.get_device_queue(physical_device.graphics_queue_family_index, 0) };

        let device_drop = DeviceDrop::new(&device);

        let allocator = Rc::new(RefCell::new(
            gpu_allocator::vulkan::Allocator::new(&gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.clone(),
                device: (*device).clone(),
                physical_device: physical_device.handle,
                debug_settings: gpu_allocator::AllocatorDebugSettings::default(),
                buffer_device_address: false,
            })
            .expect("Failed to create device allocator"),
        ));

        Self {
            info: physical_device.device_info.clone(),
            physical_device: physical_device.handle,
            device,
            allocator,
            device_drop,
            graphics_queue,
        }
    }
}

impl DeviceTrait for VulkanDevice {
    fn info(&self) -> DeviceInfo {
        self.info.clone()
    }

    fn create_buffer(&mut self, size: usize, usage: BufferUsage) -> Option<Buffer> {
        todo!()
    }

    fn create_static_buffer(&mut self, usage: BufferUsage, data: &[u8]) -> Option<Buffer> {
        todo!()
    }

    fn create_texture(&mut self, create_info: &TextureCreateInfo) -> Option<Texture> {
        todo!()
    }

    fn create_static_texture(
        &mut self,
        create_info: &TextureCreateInfo,
        data: &[u8],
    ) -> Option<Texture> {
        todo!()
    }

    fn create_sampler(&mut self, create_info: &SamplerCreateInfo) -> Option<Sampler> {
        todo!()
    }

    fn create_vertex_shader(&mut self, code: &[u8]) -> Option<VertexShader> {
        todo!()
    }

    fn create_fragment_shader(&mut self, code: &[u8]) -> Option<FragmentShader> {
        todo!()
    }

    fn create_compute_shader(&mut self, code: &[u8]) -> Option<ComputeShader> {
        todo!()
    }

    fn render_frame(
        &mut self,
        build_graph_fn: impl FnOnce(&mut RenderGraphBuilder, Option<SwapchainTexture>),
    ) {
        todo!()
    }
}
