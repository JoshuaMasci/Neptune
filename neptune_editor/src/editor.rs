use crate::gltf_loader;
use neptune_vulkan::gpu_allocator::MemoryLocation;
use neptune_vulkan::{
    vk, ColorAttachment, DepthStencilAttachment, DeviceSettings, Framebuffer, ImageAccess,
    RenderGraph, RenderPass, TransientImageDesc, TransientImageSize,
};
use std::collections::HashMap;

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

        if let Some(gltf_file) = rfd::FileDialog::new()
            .add_filter("gltf", &["gltf", "glb"])
            .set_title("pick a gltf file")
            .pick_file()
        {
            let (gltf_doc, buffers, _image_buffers) = {
                let now = std::time::Instant::now();
                let result = gltf::import(gltf_file)?;
                info!("File Loading: {}", now.elapsed().as_secs_f32());
                result
            };

            let meshes = {
                let now = std::time::Instant::now();
                let result = gltf_loader::load_meshes(&mut device, &gltf_doc, &buffers)?;
                info!("Mesh Convert/Upload: {}", now.elapsed().as_secs_f32());
                result
            };

            let mut total_vertex_count = 0;

            for mesh in meshes.iter().enumerate() {
                let vertex_count: usize =
                    mesh.1.primitives.iter().map(|prim| prim.vertex_count).sum();

                total_vertex_count += vertex_count;

                info!(
                    "Mesh({}): {} Primitives: {} Vertex: {}",
                    mesh.0,
                    mesh.1.name,
                    mesh.1.primitives.len(),
                    vertex_count,
                );
            }

            info!("Total Scene Vertex Count: {}", total_vertex_count);
        }

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
        let mut render_graph = RenderGraph::default();
        let swapchain_image = render_graph.acquire_swapchain_image(self.surface_handle);
        let depth_image = render_graph.create_transient_image(TransientImageDesc {
            size: TransientImageSize::Relative([1.0; 2], swapchain_image),
            format: vk::Format::D16_UNORM,
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            mip_levels: 1,
            memory_location: MemoryLocation::GpuOnly,
        });

        let mut image_usages = HashMap::new();
        image_usages.insert(
            swapchain_image,
            ImageAccess {
                write: true,
                stage: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                access: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                layout: vk::ImageLayout::ATTACHMENT_OPTIMAL,
            },
        );
        image_usages.insert(
            depth_image,
            ImageAccess {
                write: true,
                stage: vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                    | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
                access: vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                layout: vk::ImageLayout::ATTACHMENT_OPTIMAL,
            },
        );

        render_graph.add_pass(RenderPass {
            name: "Raster Pass".to_string(),
            queue: Default::default(),
            buffer_usages: Default::default(),
            image_usages,
            framebuffer: Some(Framebuffer {
                color_attachments: vec![ColorAttachment::new_clear(
                    swapchain_image,
                    [0.25, 0.25, 0.25, 1.0],
                )],
                depth_stencil_attachment: Some(DepthStencilAttachment::new_clear(
                    depth_image,
                    (1.0, 0),
                )),
                input_attachments: vec![],
            }),
            build_cmd_fn: None,
        });

        self.device.submit_frame(&render_graph)?;
        Ok(())
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        self.device.release_surface(self.surface_handle);
        self.instance.destroy_surface(self.surface_handle);
    }
}
