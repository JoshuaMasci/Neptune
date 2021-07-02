mod device;
mod graphics;
mod image;

use graphics::AppVersion;

use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

fn main() {
    let app = graphics::AppInfo {
        name: "Saturn Editor".to_string(),
        version: AppVersion::new(0, 0, 0),
    };

    let (mut vulkan_graphics, (event_loop, window)) = graphics::Graphics::new(&app);

    event_loop.run(move |event, _, control_flow| {
        // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
        // dispatched any events. This is ideal for games and similar applications.
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::MainEventsCleared => {
                vulkan_graphics.draw();
            }
            Event::RedrawRequested(_) => {}
            _ => (),
        }
    });
}
