use neptune_vulkan::gpu_allocator::MemoryLocation;
use neptune_vulkan::{vk, DeviceSettings};

pub struct Editor {
    instance: neptune_vulkan::Instance,
    surface_handle: neptune_vulkan::SurfaceHandle,

    device: neptune_vulkan::Device,
}

impl Editor {
    pub fn new<
        W: raw_window_handle::HasRawDisplayHandle + raw_window_handle::HasRawWindowHandle,
    >(
        window: &W,
    ) -> anyhow::Result<Self> {
        let mut instance = neptune_vulkan::Instance::new(
            &neptune_vulkan::AppInfo::new("Neptune Engine", [0, 0, 1, 0]),
            &neptune_vulkan::AppInfo::new("Neptune Editor", [0, 0, 1, 0]),
            Some(window.raw_display_handle()),
        )?;

        let surface_handle =
            instance.create_surface(window.raw_display_handle(), window.raw_window_handle())?;

        let physical_device = instance
            .select_physical_device(|physical_device| {
                if let Some(graphics_queue_index) = physical_device
                    .get_queue_family_properties()
                    .iter()
                    .enumerate()
                    .find(|(_index, queue_properties)| {
                        queue_properties.queue_flags.contains(
                            vk::QueueFlags::GRAPHICS
                                | vk::QueueFlags::COMPUTE
                                | vk::QueueFlags::TRANSFER,
                        )
                    })
                    .map(|(index, _queue_properties)| index)
                {
                    if physical_device.get_surface_support(graphics_queue_index, surface_handle) {
                        match physical_device.get_properties().device_type {
                            vk::PhysicalDeviceType::DISCRETE_GPU => Some(100),
                            vk::PhysicalDeviceType::INTEGRATED_GPU => Some(50),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .expect("Failed to find a suitable Vulkan device");

        let mut device = physical_device
            .create_device(&DeviceSettings {
                frames_in_flight: 3,
            })
            .expect("Failed to initialize vulkan device");

        device.configure_surface(
            surface_handle,
            &neptune_vulkan::SurfaceSettings {
                image_count: 3,
                format: vk::SurfaceFormatKHR {
                    format: vk::Format::B8G8R8A8_UNORM,
                    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
                },
                size: [1, 1],
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
                present_mode: vk::PresentModeKHR::FIFO,
            },
        )?;

        let buffer = device.create_buffer(
            "Test Buffer",
            &neptune_vulkan::BufferDesc {
                size: 1024,
                usage: vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                memory_location: MemoryLocation::GpuOnly,
            },
        )?;
        device.update_data_to_buffer(buffer, &vec![255; 1024])?;

        Ok(Self {
            instance,
            surface_handle,
            device,
        })
    }

    pub fn window_resize(&mut self, new_size: [u32; 2]) -> anyhow::Result<()> {
        self.device.configure_surface(
            self.surface_handle,
            &neptune_vulkan::SurfaceSettings {
                image_count: 3,
                format: vk::SurfaceFormatKHR {
                    format: vk::Format::B8G8R8A8_UNORM,
                    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
                },
                size: new_size,
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
                present_mode: vk::PresentModeKHR::FIFO,
            },
        )?;
        Ok(())
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        let mut render_graph = neptune_vulkan::RenderGraph::default();
        let swapchain_image = render_graph.acquire_swapchain_image(self.surface_handle);

        self.device.submit_frame(Some(self.surface_handle))?;
        Ok(())
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        self.device.release_surface(self.surface_handle);
        self.instance.destroy_surface(self.surface_handle);
    }
}
