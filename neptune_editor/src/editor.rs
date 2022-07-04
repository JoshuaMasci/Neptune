pub use neptune_core::log::{debug, error, info, trace, warn};

use crate::renderer::Renderer;
use crate::world::{Transform, World};
use winit::{event::*, window::Window};

pub(crate) struct Editor {
    world: World,
    renderer: Renderer,
}

impl Editor {
    pub(crate) fn new(window: &Window) -> Self {
        let mut world = World::default();

        world.entities.push(Transform {
            position: na::Vector3::new(0.0, 0.0, 10.0),
            rotation: na::UnitQuaternion::default(),
            scale: na::Vector3::new(1.0, 1.0, 1.0),
        });

        world.entities.push(Transform {
            position: na::Vector3::new(0.0, 2.0, 10.0),
            rotation: na::UnitQuaternion::default(),
            scale: na::Vector3::new(1.0, 1.0, 1.0),
        });

        Self {
            world,
            renderer: Renderer::new(window),
        }
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.renderer.resize(new_size);
    }

    #[allow(unused_variables)]
    pub(crate) fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub(crate) fn update(&mut self) {
        self.renderer.update();

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
