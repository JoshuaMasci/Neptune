use std::time::Instant;
pub use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

fn main() {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Neptune Editor")
        .with_resizable(true)
        .with_maximized(true)
        .build(&event_loop)
        .unwrap();

    let mut render_backend = neptune_core::render_backend::RenderBackend::new(&window);
    let mut imgui_layer = neptune_core::imgui_layer::ImguiLayer::new(
        &window,
        render_backend.device.clone(),
        render_backend.device_allocator.clone(),
    );

    let mut last_frame = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::NewEvents(_) => {
                imgui_layer.update_time(last_frame);
                last_frame = Instant::now();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::MainEventsCleared => {
                imgui_layer.begin_frame(&window);
                imgui_layer.end_frame(&window);

                render_backend.draw_black();
            }
            Event::RedrawRequested(_) => {}
            event => {
                imgui_layer.handle_event(&window, &event);
            }
        }
    });
}
