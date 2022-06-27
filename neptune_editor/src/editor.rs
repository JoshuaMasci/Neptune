pub use neptune_core::log::{debug, error, info, trace, warn};

use crate::renderer::Renderer;
use crate::world::World;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub(crate) struct Editor {
    world: World,
    renderer: Renderer,
}

impl Editor {
    pub(crate) fn new(window: &Window) -> Self {
        Self {
            world: Default::default(),
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
