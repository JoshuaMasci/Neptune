use neptune_core::log::{debug, error, info, trace, warn};
use neptune_graphics::{BufferUsages, MemoryType};
use std::rc::Rc;
use winit::platform::run_return::EventLoopExtRunReturn;
pub use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

include!(concat!(env!("OUT_DIR"), "/shader.rs"));

fn main() {
    neptune_core::setup_logger().expect("Failed to init logger");

    let mut event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Neptune Editor")
        .with_resizable(true)
        //.with_maximized(true)
        .build(&event_loop)
        .unwrap();

    let instance = neptune_graphics::vulkan::Instance::new(&window, "Neptune Editor", true);
    let mut device = instance.create_device(0, 3);

    let device_ref = &mut device;

    let mut test_buffer = Some(
        device_ref.create_buffer(neptune_graphics::BufferDescription {
            size: 65_536,
            usage: BufferUsages::STORAGE,
            memory_type: MemoryType::GpuOnly,
        }),
    );

    let mut test_texture = Some(
        device_ref.create_texture(neptune_graphics::TextureDescription {
            format: neptune_graphics::TextureFormat::Rgba8Unorm,
            size: neptune_graphics::TextureDimensions::D2(8_192, 8_192),
            usage: neptune_graphics::TextureUsages::SAMPLED,
            memory_type: MemoryType::GpuOnly,
        }),
    );

    let _imgui_vert_module = Rc::new(device_ref.create_shader_module(IMGUI_VERT));
    let _imgui_frag_module = Rc::new(device_ref.create_shader_module(IMGUI_FRAG));

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::NewEvents(_) => {}
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_window_id) => {
                //TODO: Render Here?
                let _ = test_buffer.take();
                let _ = test_texture.take();

                device_ref.render(move |_vulkan_render_graph| {
                    neptune_graphics::render_graph_test(_vulkan_render_graph);
                });
            }
            _event => {
                //imgui_layer.handle_event(&window, &event);
            }
        }
    });
    info!("Exiting Main Loop!");
}
