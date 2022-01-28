use neptune_core::render_graph::render_graph::RenderGraphBuilder;
use std::time::Instant;
pub use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

fn main() {
    //Test Render Graph
    neptune_core::render_graph::build_render_graph_test();

    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Neptune Editor")
        .with_resizable(true)
        .with_maximized(true)
        .build(&event_loop)
        .unwrap();

    let mut render_backend = neptune_core::render_backend::RenderBackend::new(&window);
    let mut imgui_layer =
        neptune_core::imgui_layer::ImguiLayer::new(&window, render_backend.device.clone());

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
                let _ = unsafe { render_backend.device.base.device_wait_idle() };
                *control_flow = ControlFlow::Exit
            }
            Event::MainEventsCleared => {
                let ui = imgui_layer.build_frame(&window, move |ui| {
                    ui.show_demo_window(&mut true);
                });

                let mut render_graph = RenderGraphBuilder::new();
                let output_image = imgui_layer.build_render_pass(&mut render_graph);

                render_backend.submit_render_graph(render_graph.build());
            }
            Event::RedrawRequested(_) => {}
            event => {
                imgui_layer.handle_event(&window, &event);
            }
        }
    });
}
