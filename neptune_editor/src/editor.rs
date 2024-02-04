use crate::camera::Camera;
use crate::game::entity::{Player, StaticEntity};
use crate::game::world::{World, WorldData};
use crate::gltf_loader::{load_materials, load_samplers, GltfSamplers};
use crate::input::{ButtonState, InputEventReceiver, StaticString};
use crate::material::Material;
use crate::mesh::Mesh;
use crate::platform::WindowEventReceiver;
use crate::scene::scene_renderer::{Scene, SceneCamera, SceneRenderer};
use crate::transform::Transform;
use crate::{gltf_loader, Model};
use anyhow::Context;
use glam::{Mat4, Vec3};
use neptune_vulkan::render_graph_builder::RenderGraphBuilderTrait;
use neptune_vulkan::{vk, DeviceSettings, ImageHandle};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::sync::Arc;

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
    scene_renderer: SceneRenderer,

    camera: Camera,
    camera_transform: Transform,
    scene_camera: SceneCamera,

    world: World,

    camera_move_speed: Vec3,
    camera_move_input: Vec3,

    camera_rotate_speed: Vec3,
    camera_rotate_input: Vec3,
}

impl Editor {
    const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;

    pub fn new<W: HasRawDisplayHandle + HasRawWindowHandle>(
        window: &W,
        window_size: [u32; 2],
        config: &EditorConfig,
    ) -> anyhow::Result<Self> {
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

        let surface_size = window_size;

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

        let scene_renderer = SceneRenderer::new(&mut device, Self::DEPTH_FORMAT)?;

        let gltf_scene_path = if let Some(path) = &config.gltf_scene_path {
            path.clone()
        } else {
            rfd::FileDialog::new()
                .add_filter("gltf", &["gltf", "glb"])
                .set_title("pick a gltf file")
                .pick_file()
                .expect("Failed to pick a gltf file")
        };

        let scene_camera = SceneCamera::new(&mut device)?;

        let world = load_world(&mut device, gltf_scene_path)?;

        Ok(Self {
            instance,
            surface_handle,
            surface_size,
            device,
            scene_renderer,
            camera: Default::default(),
            camera_transform: Transform::with_position(Vec3::NEG_Z),
            scene_camera,
            world,
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

    pub fn update(&mut self, delta_time: f32) {
        self.camera_transform.rotate(
            self.camera_transform.rotation * Vec3::Y,
            self.camera_rotate_speed.y * self.camera_rotate_input.y * delta_time,
        );

        self.camera_transform.translate(
            self.camera_transform.rotation
                * (self.camera_move_speed * self.camera_move_input * delta_time),
        );

        let camera_transform = match &self.world.entities.player {
            None => self.camera_transform.clone(),
            Some(player) => player.transform.clone(),
        };

        self.scene_camera.update(
            &self.camera,
            &camera_transform,
            (self.surface_size[0] as f32) / (self.surface_size[1] as f32),
        );

        self.world.update(delta_time);
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        let mut render_graph_builder =
            neptune_vulkan::basic_render_graph_builder::BasicRenderGraphBuilder::default();

        let swapchain_image = render_graph_builder.acquire_swapchain_image(self.surface_handle);

        self.scene_camera
            .write_render_passes(&mut render_graph_builder);
        self.world
            .data
            .scene
            .write_render_passes(&mut render_graph_builder);
        self.scene_renderer.write_render_passes(
            swapchain_image,
            &self.scene_camera,
            &self.world.data.scene,
            &mut render_graph_builder,
        );

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

impl WindowEventReceiver for Editor {
    fn on_window_size_changed(&mut self, new_size: [u32; 2]) -> anyhow::Result<()> {
        self.window_resize(new_size)
    }
}

impl InputEventReceiver for Editor {
    fn requests_mouse_capture(&mut self) -> bool {
        true
    }

    fn on_button_event(&mut self, button_name: StaticString, state: ButtonState) -> bool {
        false
    }

    fn on_axis_event(&mut self, axis_name: StaticString, value: f32) -> bool {
        match axis_name {
            "player_move_right_left" => {
                self.camera_move_input.x = value;
                true
            }
            "player_move_up_down" => {
                self.camera_move_input.y = value;
                true
            }
            "player_move_forward_back" => {
                self.camera_move_input.z = value;
                true
            }
            "player_move_yaw" => {
                self.camera_rotate_input.y = value;
                true
            }
            "player_move_pitch" => {
                self.camera_rotate_input.x = value;
                true
            }

            _ => false,
        }
    }

    fn on_text_event(&mut self) -> bool {
        false
    }
}

fn load_world<P: AsRef<std::path::Path>>(
    device: &mut neptune_vulkan::Device,
    path: P,
) -> anyhow::Result<World> {
    let gltf_scene = load_gltf_scene(device, path)?;

    let mut world = World {
        data: WorldData {
            scene: Scene::new(device, 1024)?,
        },
        entities: Default::default(),
    };

    for node in gltf_scene.mesh_nodes.iter() {
        let entity = StaticEntity::new(
            node.transform.into(),
            Model {
                mesh: Arc::new(gltf_scene.meshes[node.mesh_index].clone()),
                material: Arc::new(gltf_scene.materials[node.primitive_materials[0]].clone()),
            },
        );
        world.add_static_entity(entity);
    }

    //world.add_player(Player::with_position(Vec3::NEG_Z));

    Ok(world)
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
