pub use neptune_core::log::{debug, error, info, trace, warn};
use winit::event::VirtualKeyCode;

use crate::debug_camera::DebugCamera;
use crate::entity::Entity;
use crate::physics_world::Collider;
use crate::renderer::Renderer;
use crate::transform::Transform;
use crate::world::World;
use winit::window::Window;

pub(crate) struct Editor {
    last_frame: std::time::Instant,
    renderer: Renderer,
    debug_camera: DebugCamera,

    world: World,

    input: crate::game::PlayerInput,
    game_world: crate::game::World,
}

impl Editor {
    pub(crate) fn new(window: &Window) -> Self {
        let mut renderer = Renderer::new(window);
        let mut debug_camera = DebugCamera::new();

        let cube_mesh = renderer.get_mesh("resource/cube.obj").unwrap();
        let sphere_mesh = renderer.get_mesh("resource/sphere.obj").unwrap();

        let mut world = World::default();

        const SPACING: f64 = 2.5;
        let half = 128f64.sqrt() as usize;

        debug_camera.transform.position =
            glam::DVec3::new((half as f64) * 0.5 * SPACING, 0.0, -25.0);

        for x in 0..half {
            for z in 0..half {
                let (mesh, collider) = if (x + z) % 2 == 0 {
                    (cube_mesh.clone(), Collider::Box(glam::DVec3::splat(0.5)))
                } else {
                    (sphere_mesh.clone(), Collider::Sphere(0.5))
                };

                let x = x as f64;
                let z = z as f64;
                let y = x + z;

                world.add_entity(Entity::new(
                    Transform {
                        position: glam::DVec3::new(x * SPACING, y * SPACING, z * SPACING),
                        rotation: glam::Quat::IDENTITY,
                        scale: glam::Vec3::ONE,
                    },
                    Some(mesh),
                    Some(collider),
                ));
            }
        }

        Self {
            last_frame: std::time::Instant::now(),
            renderer,
            debug_camera,
            world,
            input: Default::default(),
            game_world: crate::game::World::new(),
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
        self.debug_camera.keyboard_input(key, state);
    }

    pub(crate) fn update(&mut self) {
        let delta_time = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = std::time::Instant::now();

        self.game_world.update(delta_time, &self.input);

        self.debug_camera.update(delta_time);
        self.world.update(delta_time);

        match self.renderer.render(
            &self.debug_camera.camera,
            &self.debug_camera.transform,
            &self.world,
        ) {
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
