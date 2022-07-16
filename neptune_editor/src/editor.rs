pub use neptune_core::log::{debug, error, info, trace, warn};
use winit::event::VirtualKeyCode;

use crate::renderer::Renderer;
use crate::world::{Entity, Transform, World};
use winit::window::Window;

pub(crate) struct Editor {
    last_frame: std::time::Instant,

    world: World,
    renderer: Renderer,

    linear_speed: f32,

    x_input: [bool; 2],
    y_input: [bool; 2],
    z_input: [bool; 2],
}

impl Editor {
    pub(crate) fn new(window: &Window) -> Self {
        let mut renderer = Renderer::new(window);

        let cube_mesh = renderer.get_mesh("resource/cube.obj").unwrap();

        let world_center = glam::DVec3::splat(1_000_000_000.0);

        let mut world = World::default();

        const SPACING: f64 = 2.5;
        let half = 512f64.sqrt() as usize;
        for x in 0..half {
            for y in 0..half {
                world.entities.push(Entity {
                    transform: Transform {
                        position: glam::DVec3::new(SPACING * x as f64, -1.5, SPACING * y as f64)
                            + world_center,
                        rotation: glam::Quat::default(),
                        scale: glam::Vec3::new(1.0, 1.0, 1.0),
                    },
                    mesh: cube_mesh.clone(),
                });
            }
        }

        world.camera_transform.position += world_center;

        Self {
            last_frame: std::time::Instant::now(),
            world,
            renderer,
            linear_speed: 5.0,
            x_input: [false, false],
            y_input: [false, false],
            z_input: [false, false],
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
        let pressed = state == winit::event::ElementState::Pressed;
        match key {
            VirtualKeyCode::D => {
                self.x_input[0] = pressed;
            }
            VirtualKeyCode::A => {
                self.x_input[1] = pressed;
            }
            VirtualKeyCode::Space => {
                self.y_input[0] = pressed;
            }
            VirtualKeyCode::LShift => {
                self.y_input[1] = pressed;
            }
            VirtualKeyCode::W => {
                self.z_input[0] = pressed;
            }
            VirtualKeyCode::S => {
                self.z_input[1] = pressed;
            }
            _ => {}
        }
    }

    pub(crate) fn update(&mut self) {
        let delta_time = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = std::time::Instant::now();

        let mut linear_movement = glam::Vec3::default();

        if self.x_input[0] {
            linear_movement.x += 1.0;
        }
        if self.x_input[1] {
            linear_movement.x -= 1.0;
        }

        if self.y_input[0] {
            linear_movement.y += 1.0;
        }
        if self.y_input[1] {
            linear_movement.y -= 1.0;
        }

        if self.z_input[0] {
            linear_movement.z += 1.0;
        }
        if self.z_input[1] {
            linear_movement.z -= 1.0;
        }

        linear_movement *= self.linear_speed * delta_time;

        self.world.camera_transform.position +=
            self.world.camera_transform.get_right() * linear_movement.x as f64;
        self.world.camera_transform.position +=
            self.world.camera_transform.get_up() * linear_movement.y as f64;
        self.world.camera_transform.position +=
            self.world.camera_transform.get_forward() * linear_movement.z as f64;

        match self.renderer.render(&self.world) {
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
