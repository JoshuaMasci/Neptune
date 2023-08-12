mod editor;

#[macro_use]
extern crate log;

use crate::editor::Editor;

use std::time::Instant;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

const APP_NAME: &str = "Neptune Editor";

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init_timed();

    let mut event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title(APP_NAME)
        .with_resizable(true)
        .with_inner_size(winit::dpi::PhysicalSize {
            width: 1600,
            height: 900,
        })
        .build(&event_loop)
        .unwrap();

    let mut editor = Editor::new(&window)?;

    let mut last_frame_start = Instant::now();
    let mut frame_count_time: (u32, f32) = (0, 0.0);

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                info!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                editor
                    .window_resize([new_size.width, new_size.height])
                    .expect("Failed to resize swapchain");
            }
            Event::MainEventsCleared => {
                let last_frame_time = last_frame_start.elapsed();
                last_frame_start = Instant::now();

                editor.render().expect("Failed to render a frame");

                frame_count_time.0 += 1;
                frame_count_time.1 += last_frame_time.as_secs_f32();

                if frame_count_time.1 >= 1.0 {
                    info!("FPS: {}", frame_count_time.0);
                    frame_count_time = (0, 0.0);
                }
            }
            Event::RedrawRequested(_window_id) => {}
            _ => {}
        }
    });

    info!("Exiting Main Loop!");
    Ok(())
}
