use neptune_graphics::{BufferUsages, MemoryType};
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

    {
        let vulkan_instance =
            neptune_graphics::vulkan::Instance::new(&window, "Neptune Editor", true);
        let mut vulkan_device = vulkan_instance.create_device(0, 3);

        let _ = vulkan_device.create_buffer(
            neptune_graphics::BufferDescription {
                size: 2048,
                usage: BufferUsages::STORAGE,
                memory_type: MemoryType::GpuOnly,
            },
            "Test Buffer",
        );

        let _ = vulkan_device.create_texture(
            neptune_graphics::TextureDescription {
                format: neptune_graphics::TextureFormat::Rgba8Unorm,
                size: neptune_graphics::TextureDimensions::D2(16, 16),
                usage: neptune_graphics::TextureUsages::SAMPLED,
                memory_type: MemoryType::GpuOnly,
            },
            "Test Texture",
        );
    }

    let mut last_frame = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::NewEvents(_) => {
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
                //TODO: Render Here?
            }
            Event::RedrawRequested(_) => {}
            event => {
                //imgui_layer.handle_event(&window, &event);
            }
        }
    });
}
