use crate::mesh::BoundingBox;
use crate::{gltf_loader, mesh};
use glam::{Vec3, Vec4};
use neptune_vulkan::gpu_allocator::MemoryLocation;
use neptune_vulkan::{
    vk, BufferAccess, ColorAttachment, DepthStencilAttachment, DeviceSettings, Framebuffer,
    ImageAccess, RenderGraph, RenderPass, TransientImageDesc, TransientImageSize,
};
use std::collections::HashMap;

pub struct Editor {
    instance: neptune_vulkan::Instance,
    surface_handle: neptune_vulkan::SurfaceHandle,

    device: neptune_vulkan::Device,

    raster_pipeline: neptune_vulkan::RasterPipelineHandle,
    meshes: Vec<crate::mesh::Mesh>,
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
            &neptune_vulkan::BufferDescription {
                size: 1024,
                usage: vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                memory_location: MemoryLocation::GpuOnly,
            },
        )?;
        device.update_data_to_buffer(buffer, &vec![255; 1024])?;

        // if let Some(gltf_file) = rfd::FileDialog::new()
        //     .add_filter("gltf", &["gltf", "glb"])
        //     .set_title("pick a gltf file")
        //     .pick_file()
        // {
        //     let (gltf_doc, buffers, _image_buffers) = {
        //         let now = std::time::Instant::now();
        //         let result = gltf::import(gltf_file)?;
        //         info!("File Loading: {}", now.elapsed().as_secs_f32());
        //         result
        //     };
        //
        //     let meshes = {
        //         let now = std::time::Instant::now();
        //         let result = gltf_loader::load_meshes(&mut device, &gltf_doc, &buffers)?;
        //         info!("Mesh Convert/Upload: {}", now.elapsed().as_secs_f32());
        //         result
        //     };
        //
        //     let mut total_vertex_count = 0;
        //
        //     for mesh in meshes.iter().enumerate() {
        //         let vertex_count: usize =
        //             mesh.1.primitives.iter().map(|prim| prim.vertex_count).sum();
        //
        //         total_vertex_count += vertex_count;
        //
        //         info!(
        //             "Mesh({}): {} Primitives: {} Vertex: {}",
        //             mesh.0,
        //             mesh.1.name,
        //             mesh.1.primitives.len(),
        //             vertex_count,
        //         );
        //     }
        //
        //     info!("Total Scene Vertex Count: {}", total_vertex_count);
        // }

        let raster_pipeline = {
            let vertex_shader_code =
                bytes_to_u32(include_bytes!("../resource/shader/triangle.vert.spv"));
            let fragment_shader_code =
                bytes_to_u32(include_bytes!("../resource/shader/triangle.frag.spv"));

            let vertex_state = neptune_vulkan::VertexState {
                shader_code: vertex_shader_code,
                layouts: &[
                    mesh::VertexPosition::VERTEX_BUFFER_LAYOUT,
                    mesh::VertexAttributes::VERTEX_BUFFER_LAYOUT,
                ],
            };

            device.create_raster_pipeline(&neptune_vulkan::RasterPipelineDescription {
                vertex: vertex_state,
                primitive: neptune_vulkan::PrimitiveState {
                    front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                    cull_mode: vk::CullModeFlags::NONE,
                },
                depth_state: Some(neptune_vulkan::DepthState {
                    format: vk::Format::D16_UNORM,
                    depth_enabled: true,
                    write_depth: true,
                    depth_op: vk::CompareOp::LESS,
                }),
                fragment: Some(neptune_vulkan::FragmentState {
                    shader_code: fragment_shader_code,
                    targets: &[neptune_vulkan::ColorTargetState {
                        format: vk::Format::B8G8R8A8_UNORM,
                        blend: None,
                        write_mask: vk::ColorComponentFlags::RGBA,
                    }],
                }),
            })?
        };

        let mesh = {
            let position_data = [
                Vec3::new(-0.75, 0.75, 0.5),
                Vec3::new(0.0, -0.75, 0.5),
                Vec3::new(0.75, 0.75, 0.5),
            ];
            let position_buffer = device.create_buffer_init(
                "Triangle Position Buffer",
                vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                MemoryLocation::GpuOnly,
                slice_to_bytes(&position_data),
            )?;

            let attributes_data = [
                mesh::VertexAttributes {
                    normal: Vec3::Z,
                    tangent: Vec4::new(0.0, 1.0, 0.0, 1.0),
                    tex_coords: Vec4::ZERO,
                    color: Vec4::new(1.0, 0.0, 0.0, 1.0),
                },
                mesh::VertexAttributes {
                    normal: Vec3::Z,
                    tangent: Vec4::new(0.0, 1.0, 0.0, 1.0),
                    tex_coords: Vec4::ZERO,
                    color: Vec4::new(0.0, 1.0, 0.0, 1.0),
                },
                mesh::VertexAttributes {
                    normal: Vec3::Z,
                    tangent: Vec4::new(0.0, 1.0, 0.0, 1.0),
                    tex_coords: Vec4::ZERO,
                    color: Vec4::new(0.0, 0.0, 1.0, 1.0),
                },
            ];
            let attributes_buffer = device.create_buffer_init(
                "Triangle Attribute Buffer",
                vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                MemoryLocation::GpuOnly,
                slice_to_bytes(&attributes_data),
            )?;

            mesh::Mesh {
                name: "Triangle".to_string(),
                primitives: vec![crate::mesh::Primitive {
                    bounding_box: BoundingBox::default(),
                    vertex_count: 3,
                    position_buffer,
                    attributes_buffer,
                    skinning_buffer: None,
                    index_buffer: None,
                }],
            }
        };

        Ok(Self {
            instance,
            surface_handle,
            device,
            raster_pipeline,
            meshes: vec![mesh],
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

        let mesh1 = &self.meshes[0].primitives[0];
        let mesh_buffer_handle = mesh1.position_buffer;
        let mesh_attributes_handle = mesh1.attributes_buffer;
        let mesh_vertex_count = mesh1.vertex_count as u32;
        let raster_pipeline_handle = self.raster_pipeline;

        let mut buffer_usages = HashMap::new();
        buffer_usages.insert(
            mesh_buffer_handle,
            BufferAccess {
                write: false,
                stage: vk::PipelineStageFlags2::VERTEX_ATTRIBUTE_INPUT,
                access: vk::AccessFlags2::VERTEX_ATTRIBUTE_READ,
            },
        );
        buffer_usages.insert(
            mesh_attributes_handle,
            BufferAccess {
                write: false,
                stage: vk::PipelineStageFlags2::VERTEX_ATTRIBUTE_INPUT,
                access: vk::AccessFlags2::VERTEX_ATTRIBUTE_READ,
            },
        );

        render_graph.add_pass(RenderPass {
            name: "Raster Pass".to_string(),
            queue: Default::default(),
            buffer_usages,
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
            build_cmd_fn: Some(Box::new(move |device, command_buffer, resources| unsafe {
                device.core.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[
                        resources.get_buffer(mesh_buffer_handle).handle,
                        resources.get_buffer(mesh_attributes_handle).handle,
                    ],
                    &[0, 0],
                );

                device.core.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    resources.get_raster_pipeline(raster_pipeline_handle),
                );

                device
                    .core
                    .cmd_draw(command_buffer, mesh_vertex_count, 1, 0, 0);
            })),
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

fn slice_to_bytes<T>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}

fn bytes_to_u32(slice: &[u8]) -> &[u32] {
    unsafe {
        std::slice::from_raw_parts(
            slice.as_ptr() as *const u32,
            slice.len() / std::mem::size_of::<u32>(),
        )
    }
}
