use crate::gltf_loader::{load_samplers, GltfSamplers};
use crate::mesh::Mesh;
use crate::{gltf_loader, mesh};
use neptune_vulkan::gpu_allocator::MemoryLocation;
use neptune_vulkan::{vk, DeviceSettings, ImageHandle, TransientImageDesc, TransientImageSize};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
pub struct EditorConfig {
    #[arg(short = 'p', long, value_name = "FILE")]
    gltf_scene_path: Option<std::path::PathBuf>,
}

pub struct Editor {
    instance: neptune_vulkan::Instance,
    surface_handle: neptune_vulkan::SurfaceHandle,
    surface_size: [u32; 2],

    device: neptune_vulkan::Device,

    raster_pipeline: neptune_vulkan::RasterPipelineHandle,

    view_projection_matrix_buffer: neptune_vulkan::BufferHandle,
    scene: GltfScene,
}

impl Editor {
    pub fn new(window: &winit::window::Window, config: &EditorConfig) -> anyhow::Result<Self> {
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

        let window_size = window.inner_size();
        let surface_size = [window_size.width, window_size.height];

        device.configure_surface(
            surface_handle,
            &neptune_vulkan::SurfaceSettings {
                image_count: 3,
                format: vk::SurfaceFormatKHR {
                    format: vk::Format::B8G8R8A8_UNORM,
                    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
                },
                size: surface_size,
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

        let view_projection_matrix_buffer = {
            let mut projection_matrix = glam::Mat4::perspective_infinite_lh(
                45.0f32.to_radians(),
                surface_size[0] as f32 / surface_size[1] as f32,
                0.01,
            );
            projection_matrix.y_axis.y *= -1.0;

            let view_matrix = glam::Mat4::look_to_lh(
                glam::Vec3::new(0.0, 0.0, -1.0),
                glam::Vec3::Z,
                glam::Vec3::Y,
            );

            let view_projection_matrix = projection_matrix * view_matrix;
            let matrix_data = &[view_projection_matrix];
            let matrix_data_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(
                    matrix_data.as_ptr() as *const u8,
                    std::mem::size_of_val(matrix_data),
                )
            };

            device.create_buffer_init(
                "view_projection_matrix_buffer",
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                MemoryLocation::GpuOnly,
                matrix_data_bytes,
            )?
        };

        Ok(Self {
            instance,
            surface_handle,
            surface_size,
            device,
            raster_pipeline,
            view_projection_matrix_buffer,
            scene,
        })
    }

    pub fn window_resize(&mut self, new_size: [u32; 2]) -> anyhow::Result<()> {
        self.surface_size = new_size;
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

        self.device
            .destroy_buffer(self.view_projection_matrix_buffer);
        self.view_projection_matrix_buffer = {
            let mut projection_matrix = glam::Mat4::perspective_infinite_lh(
                45.0f32.to_radians(),
                self.surface_size[0] as f32 / self.surface_size[1] as f32,
                0.01,
            );
            projection_matrix.y_axis.y *= -1.0;

            let view_matrix = glam::Mat4::look_to_lh(
                glam::Vec3::new(0.0, 0.0, -1.0),
                glam::Vec3::Z,
                glam::Vec3::Y,
            );

            let view_projection_matrix = projection_matrix * view_matrix;
            let matrix_data = &[view_projection_matrix];
            let matrix_data_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(
                    matrix_data.as_ptr() as *const u8,
                    std::mem::size_of_val(matrix_data),
                )
            };

            self.device.create_buffer_init(
                "view_projection_matrix_buffer",
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                MemoryLocation::GpuOnly,
                matrix_data_bytes,
            )?
        };

        Ok(())
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        let mut render_graph_builder =
            neptune_vulkan::render_graph_builder::RenderGraphBuilder::default();

        let swapchain_image = render_graph_builder.acquire_swapchain_image(self.surface_handle);
        let depth_image = render_graph_builder.create_transient_image(TransientImageDesc {
            size: TransientImageSize::Relative([1.0; 2], swapchain_image),
            format: vk::Format::D16_UNORM,
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            mip_levels: 1,
            memory_location: MemoryLocation::GpuOnly,
        });

        let mut raster_pass =
            neptune_vulkan::render_graph_builder::RasterPassBuilder::new("Gltf Scene")
                .add_color_attachment(
                    neptune_vulkan::render_graph_builder::ColorAttachment::new_clear(
                        swapchain_image,
                        [0.0, 0.0, 0.0, 1.0],
                    ),
                )
                .add_depth_stencil_attachment(
                    neptune_vulkan::render_graph_builder::DepthStencilAttachment::new_clear(
                        depth_image,
                        (1.0, 0),
                    ),
                );

        for mesh in self.scene.meshes.iter() {
            for primitive in mesh.primitives.iter() {
                let pipeline = self.raster_pipeline;
                let vertex_buffers = vec![
                    neptune_vulkan::render_graph_builder::BufferOffset {
                        buffer: primitive.position_buffer,
                        offset: 0,
                    },
                    neptune_vulkan::render_graph_builder::BufferOffset {
                        buffer: primitive.attributes_buffer,
                        offset: 0,
                    },
                ];

                let mut index_buffer = None;
                let dispatch = if let Some(index_buffer_ref) = &primitive.index_buffer {
                    index_buffer = Some((
                        neptune_vulkan::render_graph_builder::BufferOffset {
                            buffer: index_buffer_ref.buffer,
                            offset: 0,
                        },
                        neptune_vulkan::render_graph_builder::IndexType::U32,
                    ));

                    neptune_vulkan::render_graph_builder::RasterDispatch::DrawIndexed {
                        base_vertex: 0,
                        indices: 0..index_buffer_ref.count,
                        instances: 0..1,
                    }
                } else {
                    neptune_vulkan::render_graph_builder::RasterDispatch::Draw {
                        vertices: 0..primitive.vertex_count as u32,
                        instances: 0..1,
                    }
                };

                raster_pass = raster_pass.add_draw_command(
                    neptune_vulkan::render_graph_builder::RasterDrawCommand {
                        pipeline,
                        vertex_buffers,
                        index_buffer,
                        resources: vec![
                            neptune_vulkan::render_graph_builder::ShaderResourceUsage::StorageBuffer { buffer: self.view_projection_matrix_buffer, write: false },
                            neptune_vulkan::render_graph_builder::ShaderResourceUsage::Sampler(
                                self.scene.samplers.default,
                            ),
                            neptune_vulkan::render_graph_builder::ShaderResourceUsage::SampledImage(
                                self.scene.images[0],
                            ),
                        ],
                        dispatch,
                    },
                );
            }
        }

        render_graph_builder.add_pass(raster_pass.build());

        self.device.submit_frame(&render_graph_builder)?;
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
    images: Vec<ImageHandle>,
    samplers: GltfSamplers,
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
    let images = gltf_loader::load_images(device, &gltf_doc, &image_data)?;
    info!("Image Convert/Upload: {}", now.elapsed().as_secs_f32());

    let samplers = load_samplers(device, &gltf_doc)?;

    Ok(GltfScene {
        meshes,
        images,
        samplers,
    })
}
