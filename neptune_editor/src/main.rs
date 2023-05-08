mod imgui_layer;
mod shader;

use crate::imgui_layer::ImguiLayer;
use log::{debug, error, info, trace, warn};
use neptune_graphics::{
    MemoryType, TextureDescription, TextureDimensions, TextureFormat, TextureUsages,
};
use std::rc::Rc;
use std::time::Instant;
use winit::platform::run_return::EventLoopExtRunReturn;
pub use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

fn main() {
    pretty_env_logger::init_timed();

    neptune_graphics::render_interface_test::test_render_interface();

    let mut event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Neptune Editor")
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();

    window.set_maximized(true);

    let instance = neptune_graphics::vulkan::Instance::new(&window, "Neptune Editor", true);

    let mut device = instance.create_device(0, 3);
    let device_ref = &mut device;

    let mut imgui_layer = ImguiLayer::new(device_ref);

    let mut last_frame_start = Instant::now();
    let mut frame_count_time: (u32, f32) = (0, 0.0);

    let triangle_vertex_module = Rc::new(device_ref.create_shader_module(shader::TRIANGLE_VERT));
    let triangle_fragment_module = Rc::new(device_ref.create_shader_module(shader::TRIANGLE_FRAG));

    let test_texture = {
        let test_image = image::open("neptune_editor/resource/1k_grid.png").unwrap();
        Rc::new(device_ref.create_texture_with_data(
            TextureDescription {
                format: TextureFormat::Rgba8Unorm,
                size: TextureDimensions::D2(test_image.width(), test_image.height()),
                usage: TextureUsages::SAMPLED | TextureUsages::TRANSFER_DST,
                memory_type: MemoryType::GpuOnly,
            },
            test_image.as_rgba8().unwrap(),
        ))
    };

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

                imgui_layer.update_time(last_frame_time);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::MainEventsCleared => {
                //Build Imgui
                imgui_layer.build_frame(&window, move |ui| {
                    ui.window("Hello World1").build(|| {
                        ui.text("This...is...imgui-rs!");
                        ui.separator();
                        let mouse_pos = ui.io().mouse_pos;
                        ui.text(format!(
                            "Mouse Position: ({:.1},{:.1})",
                            mouse_pos[0], mouse_pos[1]
                        ));
                    });
                });

                let vert_module_ref = triangle_vertex_module.clone();
                let frag_module_ref = triangle_fragment_module.clone();
                let test_texture_ref = test_texture.clone();
                let imgui_ref = &mut imgui_layer;

                //Render Frame
                device_ref.render(move |render_graph| {
                    let texture_id = render_graph.import_texture(test_texture_ref);
                    let (swapchain_id, _swapchain_size) = render_graph.get_swapchain_image();
                    neptune_graphics::render_triangle_test(
                        render_graph,
                        texture_id,
                        swapchain_id,
                        vert_module_ref,
                        frag_module_ref,
                    );
                    imgui_ref.render_frame(render_graph, swapchain_id);
                });
            }
            Event::RedrawRequested(_window_id) => {}
            event => {
                imgui_layer.handle_event(&window, &event);
            }
        }
    });
    info!("Exiting Main Loop!");
}
