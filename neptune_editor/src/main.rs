mod camera;
mod debug_camera;
mod editor;
mod entity;
mod physics_world;
mod renderer;
mod transform;
mod world;

use crate::editor::Editor;
pub use neptune_core::log::{debug, error, info, trace, warn};
use winit::event_loop::EventLoop;
use winit::platform::run_return::EventLoopExtRunReturn;
pub use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

fn main() {
    entity::entity_test();

    neptune_core::setup_logger().expect("Failed to Setup Logger");

    let mut event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Neptune Editor")
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();

    window.set_maximized(true);

    let mut editor = Editor::new(&window);

    event_loop.run_return(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } => {
            if let Some(virtual_keycode) = input.virtual_keycode {
                editor.keyboard_input(virtual_keycode, input.state);
            }
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

    warn!("Exiting the Editor!");
}
