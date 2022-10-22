pub use neptune_core::log::{debug, error, info, trace, warn};

use crate::game::physics_world::Collider;
use crate::game::{Model, PlayerInput};
use crate::rendering::renderer::Renderer;
use winit::event::VirtualKeyCode;
use winit::window::Window;

pub(crate) struct Editor {
    last_frame: std::time::Instant,
    renderer: Renderer,

    input: crate::game::PlayerInput,
    game_world: crate::game::World,
}

impl Editor {
    pub(crate) fn new(window: &Window) -> Self {
        let mut renderer = Renderer::new(window);

        let normal_material = renderer.get_material("shader/normal.wgsl").unwrap();
        let red_material = renderer.get_material("shader/red.wgsl").unwrap();

        let cube_mesh = renderer.get_mesh("resource/cube.obj").unwrap();
        let sphere_mesh = renderer.get_mesh("resource/sphere.obj").unwrap();

        let cube_collider = Collider::Box(glam::DVec3::splat(1.0));
        let sphere_collider = Collider::Sphere(1.0);

        const SPACING: f64 = 2.2;
        let half = 11;

        let mut game_world = crate::game::World::new();
        game_world.add_player(crate::game::Player::new(
            crate::game::Transform {
                position: glam::DVec3::new(13.75, 20.0, -60.0),
                rotation: glam::DQuat::IDENTITY,
            },
            15.0,
        ));

        for x in 0..half {
            for z in 0..half {
                let (mesh, collider) = if (x + z) % 2 == 0 {
                    (cube_mesh.clone(), cube_collider.clone())
                } else {
                    (sphere_mesh.clone(), sphere_collider.clone())
                };

                let material = if x % 2 == 0 {
                    normal_material.clone()
                } else {
                    red_material.clone()
                };

                let x = x as f64;
                let z = z as f64;
                let y = (x + z) / 2.0;

                game_world.add_static_object(crate::game::Object {
                    transform: crate::game::Transform {
                        position: glam::DVec3::new(x * SPACING, y * SPACING, z * SPACING),
                        rotation: glam::DQuat::IDENTITY,
                    },
                    model: Some(Model { mesh, material }),
                    collider: Some(collider),
                });
            }
        }

        let start_position =
            glam::DVec3::splat((half - 2) as f64 * SPACING) + (glam::DVec3::Y * 3.0 * SPACING);
        for i in 0..10 {
            let material = if i % 2 == 0 {
                normal_material.clone()
            } else {
                red_material.clone()
            };

            game_world.add_dynamic_object(crate::game::Object {
                transform: crate::game::Transform {
                    position: start_position
                        + (glam::DVec3::Y * i as f64 * SPACING)
                        + (glam::DVec3::X * i as f64 * 0.0001),
                    rotation: glam::DQuat::IDENTITY,
                },
                model: Some(Model {
                    mesh: sphere_mesh.clone(),
                    material,
                }),
                collider: Some(sphere_collider.clone()),
            });
        }

        Self {
            last_frame: std::time::Instant::now(),
            renderer,
            input: PlayerInput::default(),
            game_world,
        }
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.renderer.resize(new_size);
    }

    pub(crate) fn keyboard_input(
        &mut self,
        key: VirtualKeyCode,
        state: winit::event::ElementState,
    ) {
        self.input.keyboard_input(key, state);
    }

    pub(crate) fn update(&mut self) {
        let delta_time = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = std::time::Instant::now();

        self.game_world.update(delta_time, &self.input);

        let (camera, transform) = self.game_world.get_camera_info();

        match self
            .renderer
            .render_game_world(&camera, &transform, &self.game_world)
        {
            Ok(_) => {}
            // Reconfigure the surface if it's lost or outdated
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.renderer.resize(self.renderer.size)
            }
            // The system is out of memory, we should probably quit
            Err(wgpu::SurfaceError::OutOfMemory) => panic!("Out of memory"),

            Err(wgpu::SurfaceError::Timeout) => warn!("Surface timeout"),
        }
    }
}
