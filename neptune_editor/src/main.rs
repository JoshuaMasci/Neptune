mod editor;
mod renderer;
mod world;

extern crate nalgebra_glm as glm;

use crate::editor::Editor;
pub use neptune_core::log::{debug, error, info, trace, warn};
use winit::event::ScanCode;
use winit::event::WindowEvent::KeyboardInput;
use winit::event_loop::EventLoop;
pub use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

fn main() {
    //IDK why wgpu has to spam the logs
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Neptune Editor")
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();

    window.set_maximized(true);

    let mut editor = Editor::new(&window);

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    device_id,
                    input,
                    is_synthetic,
                },
            ..
        } => {
            println!("Input Event");
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(physical_size),
            ..
        } => {
            editor.resize(physical_size);
        }
        Event::WindowEvent {
            event: WindowEvent::ScaleFactorChanged { new_inner_size, .. },
            ..
        } => {
            editor.resize(*new_inner_size);
        }
        Event::RedrawRequested(window_id) if window_id == window.id() => {
            editor.update();
        }
        Event::RedrawEventsCleared => {
            window.request_redraw();
        }
        _ => {}
    });
}
