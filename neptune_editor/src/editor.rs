use crate::mesh::{Mesh, Primitive};
use crate::{gltf_loader, mesh};
use neptune_vulkan::gpu_allocator::MemoryLocation;
use neptune_vulkan::{
    vk, BufferAccess, ColorAttachment, DepthStencilAttachment, DeviceSettings, Framebuffer,
    ImageAccess, ImageHandle, RenderGraph, RenderPass, TransientImageDesc, TransientImageSize,
};
use std::collections::HashMap;

#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
pub struct EditorConfig {
    #[arg(short = 'p', long, value_name = "FILE")]
    gltf_scene_path: Option<std::path::PathBuf>,
}

pub struct Editor {
    instance: neptune_vulkan::Instance,
    surface_handle: neptune_vulkan::SurfaceHandle,

    device: neptune_vulkan::Device,

    raster_pipeline: neptune_vulkan::RasterPipelineHandle,

    scene: GltfScene,
}

impl Editor {
    pub fn new<
        W: raw_window_handle::HasRawDisplayHandle + raw_window_handle::HasRawWindowHandle,
    >(
        window: &W,
        config: &EditorConfig,
    ) -> anyhow::Result<Self> {
        let mut instance = neptune_vulkan::Instance::new(
            &neptune_vulkan::AppInfo::new("Neptune Engine", [0, 0, 1, 0]),
            &neptune_vulkan::AppInfo::new(crate::APP_NAME, [0, 0, 1, 0]),
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

        let raster_pipeline = {
            let vertex_shader_code = crate::shader::MESH_STATIC_VERT;
            let fragment_shader_code = crate::shader::MESH_FRAG;

            let vertex_state = neptune_vulkan::VertexState {
                shader: neptune_vulkan::ShaderStage {
                    code: vertex_shader_code,
                    entry: "main",
                },
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
                    shader: neptune_vulkan::ShaderStage {
                        code: fragment_shader_code,
                        entry: "main",
                    },
                    targets: &[neptune_vulkan::ColorTargetState {
                        format: vk::Format::B8G8R8A8_UNORM,
                        blend: None,
                        write_mask: vk::ColorComponentFlags::RGBA,
                    }],
                }),
            })?
        };

        let gltf_scene_path = if let Some(path) = &config.gltf_scene_path {
            path.clone()
        } else {
            rfd::FileDialog::new()
                .add_filter("gltf", &["gltf", "glb"])
                .set_title("pick a gltf file")
                .pick_file()
                .expect("Failed to pick a gltf file")
        };

        let scene = load_gltf_scene(&mut device, &gltf_scene_path)?;

        Ok(Self {
            instance,
            surface_handle,
            device,
            raster_pipeline,
            scene,
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

        let mut buffer_usages = HashMap::new();
        let mut primitives: Vec<Primitive> = Vec::new();

        for mesh in self.scene.meshes.iter() {
            for primitive in mesh.primitives.iter() {
                buffer_usages.insert(
                    primitive.position_buffer,
                    BufferAccess {
                        write: false,
                        stage: vk::PipelineStageFlags2::VERTEX_ATTRIBUTE_INPUT,
                        access: vk::AccessFlags2::VERTEX_ATTRIBUTE_READ,
                    },
                );
                buffer_usages.insert(
                    primitive.attributes_buffer,
                    BufferAccess {
                        write: false,
                        stage: vk::PipelineStageFlags2::VERTEX_ATTRIBUTE_INPUT,
                        access: vk::AccessFlags2::VERTEX_ATTRIBUTE_READ,
                    },
                );

                if let Some(index_buffer) = &primitive.index_buffer {
                    buffer_usages.insert(
                        index_buffer.buffer,
                        BufferAccess {
                            write: false,
                            stage: vk::PipelineStageFlags2::INDEX_INPUT_KHR,
                            access: vk::AccessFlags2::INDEX_READ,
                        },
                    );
                }

                primitives.push(primitive.clone());
            }
        }

        let raster_pipeline_handle = self.raster_pipeline;
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
                device.core.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    resources.get_raster_pipeline(raster_pipeline_handle),
                );

                {
                    let image_size = resources.get_image(swapchain_image).size;
                    let image_size = [image_size.width as f32, image_size.height as f32];
                    let mut projection_matrix = glam::Mat4::perspective_infinite_lh(
                        45.0f32.to_radians(),
                        image_size[0] / image_size[1],
                        0.01,
                    );
                    projection_matrix.y_axis.y *= -1.0;

                    let view_matrix = glam::Mat4::look_to_lh(
                        glam::Vec3::new(0.0, 0.0, -1.0),
                        glam::Vec3::Z,
                        glam::Vec3::Y,
                    );
                    let model_matrix = glam::Mat4::IDENTITY;

                    let view_projection_matrix = projection_matrix * view_matrix;

                    let push_data = &[view_projection_matrix, model_matrix];
                    let push_data_bytes: &[u8] = std::slice::from_raw_parts(
                        push_data.as_ptr() as *const u8,
                        std::mem::size_of_val(push_data),
                    );

                    device.core.cmd_push_constants(
                        command_buffer,
                        resources.get_pipeline_layout(),
                        vk::ShaderStageFlags::ALL,
                        0,
                        push_data_bytes,
                    );

                    device.core.cmd_push_constants(
                        command_buffer,
                        resources.get_pipeline_layout(),
                        vk::ShaderStageFlags::ALL,
                        push_data_bytes.len() as u32,
                        &0u32.to_ne_bytes(),
                    );
                }

                for primitive in primitives.iter() {
                    device.core.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[
                            resources.get_buffer(primitive.position_buffer).handle,
                            resources.get_buffer(primitive.attributes_buffer).handle,
                        ],
                        &[0, 0],
                    );

                    if let Some(index_buffer) = &primitive.index_buffer {
                        device.core.cmd_bind_index_buffer(
                            command_buffer,
                            resources.get_buffer(index_buffer.buffer).handle,
                            0,
                            vk::IndexType::UINT32,
                        );

                        device.core.cmd_draw_indexed(
                            command_buffer,
                            index_buffer.count,
                            1,
                            0,
                            0,
                            0,
                        );
                    } else {
                        device.core.cmd_draw(
                            command_buffer,
                            primitive.vertex_count as u32,
                            1,
                            0,
                            0,
                        );
                    }
                }
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

struct GltfScene {
    meshes: Vec<Mesh>,
    textures: Vec<ImageHandle>,
}

fn load_gltf_scene<P: AsRef<std::path::Path>>(
    device: &mut neptune_vulkan::Device,
    path: P,
) -> anyhow::Result<GltfScene> {
    let (gltf_doc, buffer_data, image_data) = {
        let now = std::time::Instant::now();
        let result = gltf::import(path)?;
        info!("File Loading: {}", now.elapsed().as_secs_f32());
        result
    };

    let now = std::time::Instant::now();
    let meshes = gltf_loader::load_meshes(device, &gltf_doc, &buffer_data)?;
    info!("Mesh Convert/Upload: {}", now.elapsed().as_secs_f32());

    let now = std::time::Instant::now();
    let textures = gltf_loader::load_textures(device, &gltf_doc, &image_data)?;
    info!("Texture Convert/Upload: {}", now.elapsed().as_secs_f32());

    Ok(GltfScene { meshes, textures })
}
