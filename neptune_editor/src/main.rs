use neptune_core::render_graph::render_graph::RenderGraphBuilder;
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
    let mut imgui_layer =
        neptune_core::imgui_layer::ImguiLayer::new(&window, render_backend.device.clone());
    let mut scene_layer = neptune_core::scene_layer::SceneLayer::new(render_backend.device.clone());

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
                imgui_layer.build_frame(&window, move |ui| {
                    let _dock_space_id = neptune_core::imgui_docking::enable_docking();

                    ui.window("Hello World1").build(|| {
                        ui.text("Hello world!");
                        ui.text("こんにちは世界！");
                        ui.text("This...is...imgui-rs!");
                        ui.separator();
                        let mouse_pos = ui.io().mouse_pos;
                        ui.text(format!(
                            "Mouse Position: ({:.1},{:.1})",
                            mouse_pos[0], mouse_pos[1]
                        ));
                    });

                    ui.window("Hello World2").build(|| {
                        ui.text("Hello world!");
                    });

                    ui.window("Goodbye World").build(|| {
                        ui.text("Goodbye World");
                    });
                });

                let mut render_graph = RenderGraphBuilder::new();
                let swapchain_image = render_graph.get_swapchain_image_resource();
                imgui_layer.build_render_pass(&mut render_graph, swapchain_image);
                scene_layer.build_render_pass(&mut render_graph, swapchain_image);
                if !render_backend.submit_render_graph(render_graph.build()) {
                    imgui_layer.end_frame_no_render();
                }
            }
            Event::RedrawRequested(_) => {}
            event => {
                imgui_layer.handle_event(&window, &event);
            }
        }
    });
}
