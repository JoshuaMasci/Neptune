mod scene_layer;

use neptune_core::log::{debug, error, info, trace, warn};
use std::time::Instant;
use winit::platform::run_return::EventLoopExtRunReturn;
pub use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

fn main() {
    neptune_core::setup_logger().expect("Failed to init logger");

    let mut event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Neptune Editor")
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();

    window.set_maximized(true);

    neptune_graphics::test_api(&window);

    let mut last_frame_start = Instant::now();
    let mut frame_count_time: (u32, f32) = (0, 0.0);

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::NewEvents(_) => {
                let last_frame_time = last_frame_start.elapsed();
                last_frame_start = Instant::now();

                frame_count_time.0 += 1;
                frame_count_time.1 += last_frame_time.as_secs_f32();

                if frame_count_time.1 >= 1.0 {
                    info!("FPS: {}", frame_count_time.0);
                    frame_count_time = (0, 0.0);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                info!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::MainEventsCleared => {
                // device.render_frame(|render_graph_builder| {
                // });
            }
            Event::RedrawRequested(_window_id) => {}
            _ => {}
        }
    });
    info!("Exiting Main Loop!");
}

//TODO: verify that this works
fn to_bytes_unsafe<T>(data: &[T]) -> &[u8] {
    let ptr = data.as_ptr();
    let ptr_size = std::mem::size_of::<T>() * data.len();
    unsafe { &*std::ptr::slice_from_raw_parts(ptr as *const u8, ptr_size) }
}
