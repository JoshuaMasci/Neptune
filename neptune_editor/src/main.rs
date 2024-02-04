mod camera;
mod editor;
mod game;
mod gltf_loader;
mod input;
mod input_system;
mod material;
mod mesh;
mod platform;
mod scene;
mod shader;
mod transform;

#[macro_use]
extern crate log;

use crate::editor::{Editor, EditorConfig};
use std::sync::Arc;

use crate::material::Material;
use crate::mesh::Mesh;
use clap::Parser;
use std::time::Instant;
use winit::window::CursorGrabMode;
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

//TODO: create these
#[derive(Clone)]
pub struct Model {
    pub mesh: Arc<Mesh>,
    pub material: Arc<Material>,
}

pub const APP_NAME: &str = "Neptune Editor";

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init_timed();

    let mut input = winit_input_helper::WinitInputHelper::new();
    let event_loop = winit::event_loop::EventLoop::new()?;

    let window_size = [1600, 900];
    let window = winit::window::WindowBuilder::new()
        .with_title(APP_NAME)
        .with_resizable(true)
        .with_inner_size(winit::dpi::PhysicalSize {
            width: window_size[0],
            height: window_size[1],
        })
        .build(&event_loop)
        .unwrap();

    let mut editor = Editor::new(&window, window_size, &EditorConfig::parse())?;

    let mut last_frame_start = Instant::now();
    let mut frame_count_time: (u32, f32) = (0, 0.0);

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run(move |event, window_target| {
        input.update(&event);
        editor.winit_input.on_event(&event);
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                info!("The close button was pressed; stopping");
                window_target.exit();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                editor
                    .window_resize([new_size.width, new_size.height])
                    .expect("Failed to resize swapchain");
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event, .. },
                ..
            } => {
                // Hardcode mouse free on ESC
                if event.physical_key
                    == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape)
                    && event.state == winit::event::ElementState::Pressed
                {
                    let _ = window.set_cursor_grab(CursorGrabMode::None);
                    window.set_cursor_visible(true);
                }
            }

            Event::AboutToWait => {
                let last_frame_time = last_frame_start.elapsed();
                last_frame_start = Instant::now();

                editor.process_input(&input);
                editor.update(last_frame_time.as_secs_f32());

                editor.render().expect("Failed to render a frame");

                frame_count_time.0 += 1;
                frame_count_time.1 += last_frame_time.as_secs_f32();

                if frame_count_time.1 >= 1.0 {
                    info!("FPS: {}", frame_count_time.0);
                    frame_count_time = (0, 0.0);
                }
            }
            _ => {}
        }
    })?;
    info!("Exiting Main Loop!");
    Ok(())
}
