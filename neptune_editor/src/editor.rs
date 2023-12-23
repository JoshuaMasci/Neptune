use crate::camera::Camera;
use crate::gltf_loader::{load_materials, load_samplers, GltfSamplers};
use crate::material::Material;
use crate::mesh::Mesh;
use crate::transform::Transform;
use crate::{gltf_loader, mesh};
use anyhow::Context;
use glam::{BVec3, Mat4, Vec3};
use neptune_vulkan::gpu_allocator::MemoryLocation;
use neptune_vulkan::render_graph_builder::{BufferWriteCallback, RenderGraphBuilderTrait};
use neptune_vulkan::{vk, DeviceSettings, ImageHandle, TransientImageDesc, TransientImageSize};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;

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
    fullscreen_copy_pipeline: neptune_vulkan::RasterPipelineHandle,

    view_projection_matrix_buffer: neptune_vulkan::BufferHandle,
    model_matrices_buffer: neptune_vulkan::BufferHandle,
    scene: GltfScene,

    camera_transform: Transform,
    camera: Camera,

    camera_move_speed: Vec3,
    camera_move_input: Vec3,

    camera_rotate_speed: Vec3,
    camera_rotate_input: Vec3,
}

impl Editor {
    pub fn new(window: &winit::window::Window, config: &EditorConfig) -> anyhow::Result<Self> {
        let raw_display_handle = window.raw_display_handle();
        let raw_window_handle = window.raw_window_handle();

        let mut instance = neptune_vulkan::Instance::new(
            &neptune_vulkan::AppInfo::new("Neptune Engine", [0, 0, 1, 0]),
            &neptune_vulkan::AppInfo::new(crate::APP_NAME, [0, 0, 1, 0]),
            Some(raw_display_handle),
        )?;

        let surface_handle = instance.create_surface(raw_display_handle, raw_window_handle)?;

        let physical_device = instance
            .select_physical_device(Some(surface_handle), |physical_device| {
                //Must support graphics and be an known gpu type
                if !physical_device.supports_graphics()
                    || physical_device.info.device_type
                        == neptune_vulkan::PhysicalDeviceType::Unknown
                {
                    return 0;
                }

                const DISCRETE_DEVICE_ADJUSTMENT: usize = 100;
                const MAX_MEMORY_CONSIDERATION: usize = 25;
                const BYTES_TO_GIGABYTES: usize = 1024 * 1024 * 1024;
                const ASYNC_COMPUTE: usize = 25;
                const ASYNC_TRANSFER: usize = 25;

                let mut score = 0;

                // Preferred Discrete GPU's
                if physical_device.info.device_type == neptune_vulkan::PhysicalDeviceType::Discrete
                {
                    score += DISCRETE_DEVICE_ADJUSTMENT;
                }

                // Prefer async compute support
                if physical_device.supports_async_compute() {
                    score += ASYNC_COMPUTE;
                }

                // Prefer async transfer support
                if physical_device.supports_async_transfer() {
                    score += ASYNC_TRANSFER;
                }

                // Prefer more memory
                score += (physical_device.memory.device_local_bytes / BYTES_TO_GIGABYTES)
                    .max(MAX_MEMORY_CONSIDERATION);

                score
            })
            .context("Failed to find a suitable Vulkan device")?;

        info!("Selected Device: {:#?}", physical_device);

        const FRAME_IN_FLIGHT_COUNT: u32 = 3;

        let mut device = physical_device
            .create_device(DeviceSettings {
                frames_in_flight: FRAME_IN_FLIGHT_COUNT,
            })
            .context("Failed to initialize vulkan device")?;

        let window_size = window.inner_size();
        let surface_size = [window_size.width, window_size.height];

        device.configure_surface(
            surface_handle,
            &neptune_vulkan::SurfaceSettings {
                image_count: FRAME_IN_FLIGHT_COUNT,
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
                    cull_mode: vk::CullModeFlags::BACK,
                },
                depth_state: Some(neptune_vulkan::DepthState {
                    format: vk::Format::D32_SFLOAT,
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

        let fullscreen_copy_pipeline =
            device.create_raster_pipeline(&neptune_vulkan::RasterPipelineDescription {
                vertex: neptune_vulkan::VertexState {
                    shader: neptune_vulkan::ShaderStage {
                        code: crate::shader::FULLSCREEN_QUAD_VERT,
                        entry: "main",
                    },
                    layouts: &[],
                },
                primitive: neptune_vulkan::PrimitiveState {
                    front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                    cull_mode: vk::CullModeFlags::NONE,
                },
                depth_state: None,
                fragment: Some(neptune_vulkan::FragmentState {
                    shader: neptune_vulkan::ShaderStage {
                        code: crate::shader::FULLSCREEN_STORAGE_COPY_FRAG,
                        entry: "main",
                    },
                    targets: &[neptune_vulkan::ColorTargetState {
                        format: vk::Format::B8G8R8A8_UNORM,
                        blend: None,
                        write_mask: vk::ColorComponentFlags::RGBA,
                    }],
                }),
            })?;

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

        let view_projection_matrix_buffer = device.create_buffer(
            "view_projection_matrix_buffer",
            std::mem::size_of::<Mat4>(),
            neptune_vulkan::BufferUsage::STORAGE | neptune_vulkan::BufferUsage::TRANSFER,
            MemoryLocation::GpuOnly,
        )?;

        let model_matrices_buffer = {
            let model_matrices_data: Vec<Mat4> =
                scene.mesh_nodes.iter().map(|node| node.transform).collect();
            let model_matrices_slice = model_matrices_data.as_slice();
            let model_matrices_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(
                    model_matrices_slice.as_ptr() as *const u8,
                    std::mem::size_of_val(model_matrices_slice),
                )
            };

            device.create_buffer_init(
                "model_matrices_buffer",
                neptune_vulkan::BufferUsage::STORAGE | neptune_vulkan::BufferUsage::TRANSFER,
                MemoryLocation::GpuOnly,
                model_matrices_bytes,
            )?
        };

        Ok(Self {
            instance,
            surface_handle,
            surface_size,
            device,
            raster_pipeline,
            fullscreen_copy_pipeline,
            view_projection_matrix_buffer,
            model_matrices_buffer,
            scene,
            camera_transform: Transform::with_position(Vec3::NEG_Z),
            camera: Camera::default(),
            camera_move_speed: Vec3::splat(1.0),
            camera_move_input: Vec3::ZERO,
            camera_rotate_speed: Vec3::new(0.0, 60.0f32.to_radians(), 0.0),
            camera_rotate_input: Vec3::ZERO,
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

        Ok(())
    }

    pub fn process_input(&mut self, input: &WinitInputHelper) {
        fn buttons_to_axis(input: &WinitInputHelper, pos_key: KeyCode, neg_key: KeyCode) -> f32 {
            let mut value = 0.0;
            if input.key_held(pos_key) {
                value += 1.0;
            }

            if input.key_held(neg_key) {
                value -= 1.0;
            }

            value
        }

        self.camera_move_input.x = buttons_to_axis(input, KeyCode::KeyD, KeyCode::KeyA);
        self.camera_move_input.y = buttons_to_axis(input, KeyCode::ShiftLeft, KeyCode::ControlLeft);
        self.camera_move_input.z = buttons_to_axis(input, KeyCode::KeyW, KeyCode::KeyS);

        self.camera_rotate_input.y =
            buttons_to_axis(input, KeyCode::ArrowRight, KeyCode::ArrowLeft);
    }

    pub fn update(&mut self, delta_time: f32) {
        self.camera_transform.rotate(
            self.camera_transform.rotation * Vec3::Y,
            self.camera_rotate_speed.y * self.camera_rotate_input.y * delta_time,
        );

        self.camera_transform.translate(
            self.camera_transform.rotation
                * (self.camera_move_speed * self.camera_move_input * delta_time),
        );
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        let mut render_graph_builder =
            neptune_vulkan::basic_render_graph_builder::BasicRenderGraphBuilder::default();

        let swapchain_image = render_graph_builder.acquire_swapchain_image(self.surface_handle);
        let depth_image = render_graph_builder.create_transient_image(TransientImageDesc {
            size: TransientImageSize::Relative([1.0; 2], swapchain_image),
            format: vk::Format::D32_SFLOAT,
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            mip_levels: 1,
            memory_location: MemoryLocation::GpuOnly,
        });

        {
            let aspect_ratio = self.surface_size[0] as f32 / self.surface_size[1] as f32;
            let projection_matrix = self.camera.projection_matrix(aspect_ratio);
            let view_matrix = self.camera_transform.view_matrix();
            let view_projection_matrix = projection_matrix * view_matrix;

            let buffer_size = std::mem::size_of_val(&view_projection_matrix);
            let host_view_projection_matrix_buffer = render_graph_builder.create_transient_buffer(
                buffer_size,
                neptune_vulkan::BufferUsage::TRANSFER,
                MemoryLocation::CpuToGpu,
            );

            render_graph_builder.add_mapped_buffer_write(
                host_view_projection_matrix_buffer,
                BufferWriteCallback::new(move |slice| {
                    let matrix_data = &[view_projection_matrix];
                    let matrix_data_bytes: &[u8] = unsafe {
                        std::slice::from_raw_parts(
                            matrix_data.as_ptr() as *const u8,
                            std::mem::size_of_val(matrix_data),
                        )
                    };
                    slice.copy_from_slice(matrix_data_bytes);
                }),
            );

            let mut data_upload_pass =
                neptune_vulkan::render_graph_builder::TransferPassBuilder::new(
                    "Matrix Upload Pass",
                    neptune_vulkan::render_graph::QueueType::Graphics,
                );
            data_upload_pass.copy_buffer_to_buffer(
                neptune_vulkan::render_graph_builder::BufferOffset {
                    buffer: host_view_projection_matrix_buffer,
                    offset: 0,
                },
                neptune_vulkan::render_graph_builder::BufferOffset {
                    buffer: self.view_projection_matrix_buffer,
                    offset: 0,
                },
                buffer_size,
            );
            data_upload_pass.build(&mut render_graph_builder);
        }

        let mut raster_pass_builder =
            neptune_vulkan::render_graph_builder::RasterPassBuilder::new("Swapchain Pass");
        raster_pass_builder.add_color_attachment(swapchain_image, Some([0.0, 0.0, 0.0, 1.0]));
        raster_pass_builder.add_depth_stencil_attachment(depth_image, Some((1.0, 0)));

        for (index, node) in self.scene.mesh_nodes.iter().enumerate() {
            let mesh = &self.scene.meshes[node.mesh_index];
            for (primitive, material_index) in
                mesh.primitives.iter().zip(node.primitive_materials.iter())
            {
                let (base_color_texture, base_color_sampler) = self.scene.materials
                    [*material_index]
                    .base_color_texture
                    .as_ref()
                    .map(|tex| (tex.image, tex.sampler))
                    .unwrap_or((self.scene.images[0], self.scene.samplers.default));

                let mut draw_command_builder =
                    neptune_vulkan::render_graph_builder::RasterDrawCommandBuilder::new(
                        self.raster_pipeline,
                    );

                draw_command_builder.add_vertex_buffer(
                    neptune_vulkan::render_graph_builder::BufferOffset {
                        buffer: primitive.position_buffer,
                        offset: 0,
                    },
                );
                draw_command_builder.add_vertex_buffer(
                    neptune_vulkan::render_graph_builder::BufferOffset {
                        buffer: primitive.attributes_buffer,
                        offset: 0,
                    },
                );
                draw_command_builder.read_buffer(self.view_projection_matrix_buffer);
                draw_command_builder.read_buffer(self.model_matrices_buffer);
                draw_command_builder.read_sampler(base_color_sampler);
                draw_command_builder.read_sampled_image(base_color_texture);

                let instance_range = (index as u32)..(index as u32 + 1);

                if let Some(index_buffer_ref) = &primitive.index_buffer {
                    draw_command_builder.draw_indexed(
                        0,
                        0..index_buffer_ref.count,
                        instance_range,
                        neptune_vulkan::render_graph_builder::BufferOffset {
                            buffer: index_buffer_ref.buffer,
                            offset: 0,
                        },
                        neptune_vulkan::render_graph::IndexType::U32,
                    );
                } else {
                    draw_command_builder.draw(0..primitive.vertex_count as u32, instance_range);
                }

                draw_command_builder.build(&mut raster_pass_builder);
            }
        }

        raster_pass_builder.build(&mut render_graph_builder);

        let render_graph = render_graph_builder.build();
        self.device.submit_graph(&render_graph)?;
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
    materials: Vec<Material>,

    mesh_nodes: Vec<GltfNode>,
}

struct GltfNode {
    transform: Mat4,
    mesh_index: usize,
    primitive_materials: Vec<usize>,
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

    let materials = load_materials(&gltf_doc, &images, &samplers);

    let mut mesh_nodes = Vec::new();

    for root_node in gltf_doc.default_scene().unwrap().nodes() {
        gltf_node(Mat4::IDENTITY, &mut mesh_nodes, &root_node);
    }

    Ok(GltfScene {
        meshes,
        images,
        samplers,
        materials,
        mesh_nodes,
    })
}

fn gltf_node(parent_transform: Mat4, mesh_nodes: &mut Vec<GltfNode>, node: &gltf::Node) {
    let local_transform: Mat4 = Mat4::from_cols_array_2d(&node.transform().matrix());
    let world_transform = parent_transform * local_transform;

    if let Some(mesh) = node.mesh() {
        mesh_nodes.push(GltfNode {
            transform: world_transform,
            mesh_index: mesh.index(),
            primitive_materials: mesh
                .primitives()
                .map(|primitive| primitive.material().index().unwrap())
                .collect(),
        });
    }

    for child in node.children() {
        gltf_node(world_transform, mesh_nodes, &child);
    }
}
