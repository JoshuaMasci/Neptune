use crate::camera::{Camera, FieldOfView};
use crate::game::entity::StaticEntity;
use crate::game::player::Player;
use crate::game::ship::{Module, ModuleType, Ship};
use crate::game::world::{World, WorldData};
use crate::gltf_loader::load_gltf_scene;
use crate::input::{ButtonState, InputEventReceiver, StaticString};
use crate::physics::physics_world::{Collider, PhysicsWorld};
use crate::platform::WindowEventReceiver;
use crate::scene::scene_renderer::{Model, ModelPrimitive, Scene, SceneCamera, SceneRenderer};
use crate::transform::Transform;
use anyhow::Context;
use glam::Vec3;
use neptune_vulkan::render_graph_builder::RenderGraphBuilderTrait;
use neptune_vulkan::{vk, DeviceSettings, SurfaceHandle};
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
        clear_surfaces(&mut device, [0.0; 3], &[surface_handle])?;

        let scene_renderer = SceneRenderer::new(&mut device, Self::DEPTH_FORMAT)?;

        // let gltf_scene_path = if let Some(path) = &config.gltf_scene_path {
        //     path.clone()
        // } else {
        //     rfd::FileDialog::new()
        //         .add_filter("gltf", &["gltf", "glb"])
        //         .set_title("pick a gltf file")
        //         .pick_file()
        //         .expect("Failed to pick a gltf file")
        // };

        let scene_camera = SceneCamera::new(&mut device)?;

        //let world = load_world(&mut device, gltf_scene_path)?;
        let world = create_test_world(&mut device)?;

        Ok(Self {
            instance,
            surface_handle,
            surface_size,
            device,
            scene_renderer,
            camera: Camera::new(FieldOfView::X(90.0), 0.1, None),
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
        info!("Swapchain Resize: {:?}", new_size);
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
            Some(player) => player.get_camera_transform(),
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
        if let Some(player) = &mut self.world.entities.player {
            return player.on_button_event(button_name, state);
        }

        false
    }

    fn on_axis_event(&mut self, axis_name: StaticString, value: f32) -> bool {
        if let Some(player) = &mut self.world.entities.player {
            return player.on_axis_event(axis_name, value);
        }

        match axis_name {
            "player_move_left_right" => {
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

    fn on_text_event(&mut self, text: String) -> bool {
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
            physics: PhysicsWorld::new(),
        },
        entities: Default::default(),
    };

    for node in gltf_scene.mesh_nodes.iter() {
        let model = Model {
            name: gltf_scene.meshes[node.mesh_index].name.clone(),
            primitives: gltf_scene.meshes[node.mesh_index]
                .primitives
                .iter()
                .zip(node.primitive_materials.iter())
                .map(|(primitive, material_index)| ModelPrimitive {
                    primitive: primitive.clone(),
                    material: gltf_scene
                        .materials
                        .get(*material_index)
                        .cloned()
                        .map(Arc::new),
                })
                .collect(),
        };

        let entity = StaticEntity::new(node.transform.into(), model, None);
        world.add_static_entity(entity);
    }

    world.add_player(Player::with_position(Vec3::NEG_Z));

    Ok(world)
}

fn create_test_world(device: &mut neptune_vulkan::Device) -> anyhow::Result<World> {
    let gltf_scene = load_gltf_scene(device, "neptune_editor/resource/PurpleCube2.glb")?;

    let mut world = World {
        data: WorldData {
            scene: Scene::new(device, 1024)?,
            physics: PhysicsWorld::new(),
        },
        entities: Default::default(),
    };

    let model = Model {
        name: "Ground".to_string(),
        primitives: vec![ModelPrimitive {
            primitive: gltf_scene.meshes[0].primitives[0].clone(),
            material: gltf_scene.materials.first().cloned().map(Arc::new),
        }],
    };
    let ground_size = Vec3::new(8.0, 0.5, 8.0);
    let ground_entity = StaticEntity::new(
        Transform {
            position: Vec3::NEG_Y * 0.5,
            scale: ground_size,
            ..Default::default()
        },
        model.clone(),
        Some(Collider::Box(ground_size)),
    );
    world.add_static_entity(ground_entity);

    world.add_player(Player::with_position(Vec3::Y * 3.0));

    //Ship
    {
        let module = Module {
            model: model.clone(),
            collider: Collider::Box(Vec3::splat(0.5)),
        };

        let ship = Ship {
            connector_module: module.clone(),
            hallway_module: module.clone(),
            room_module: module.clone(),
            module_list: vec![
                (Transform::default(), ModuleType::Connector),
                (
                    Transform::with_position(Vec3::Y * 2.0),
                    ModuleType::Connector,
                ),
                (
                    Transform::with_position(Vec3::Y * 4.0),
                    ModuleType::Connector,
                ),
            ],
            transform: Transform::with_position(Vec3::Y * 5.0 + Vec3::Z * 2.0),
            rigid_body_handle: None,
            modules: vec![],
        };
        world.add_ship(ship);
    }

    Ok(world)
}

/// Simple Render Graph to clear the screen before asset loading happens
fn clear_surfaces(
    device: &mut neptune_vulkan::Device,
    color: [f32; 3],
    surface_handles: &[SurfaceHandle],
) -> anyhow::Result<()> {
    let mut render_graph_builder =
        neptune_vulkan::basic_render_graph_builder::BasicRenderGraphBuilder::default();

    for handle in surface_handles {
        let swapchain_image = render_graph_builder.acquire_swapchain_image(*handle);
        let mut raster_pass_builder =
            neptune_vulkan::render_graph_builder::RasterPassBuilder::new("Swapchain Pass");
        raster_pass_builder
            .add_color_attachment(swapchain_image, Some([color[0], color[1], color[2], 1.0]));
        raster_pass_builder.build(&mut render_graph_builder);
    }

    let render_graph = render_graph_builder.build();
    device.submit_graph(&render_graph)?;
    Ok(())
}
