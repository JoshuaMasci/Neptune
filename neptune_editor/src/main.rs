#[macro_use]
extern crate log;

use neptune_graphics::AppInfo;
use neptune_vulkan::ash::vk::Format;
use neptune_vulkan::{
    AddressMode, BufferUsage, ColorAttachment, CompositeAlphaMode, DepthStencilAttachment,
    FilterMode, PresentMode, SamplerCreateInfo, TextureSize, TextureUsage,
};
use std::time::Instant;
use winit::platform::run_return::EventLoopExtRunReturn;
pub use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

const APP_NAME: &str = "Neptune Editor";

fn main() {
    pretty_env_logger::init_timed();

    let mut event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title(APP_NAME)
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();
    window.set_maximized(true);

    {
        let new_instance = neptune_graphics::create_vulkan_instance(
            &AppInfo::new("Neptune Engine", [0, 0, 1, 0]),
            &AppInfo::new(APP_NAME, [0, 0, 1, 0]),
        );

        let selected_device = new_instance
            .select_and_create_device(None, |index, device_info| {
                println!("{}: {:#?}", index, device_info);
                match device_info.device_type {
                    neptune_graphics::DeviceType::Integrated => Some(50),
                    neptune_graphics::DeviceType::Discrete => Some(100),
                    neptune_graphics::DeviceType::Unknown => None,
                }
            })
            .unwrap();

        let device_info = selected_device.info();
        println!("Selected: {:#?}", device_info);
        let create_info = neptune_graphics::DeviceCreateInfo {
            frames_in_flight_count: 3,
            features: device_info.features,
            extensions: device_info.extensions,
        };

        let device = selected_device.create(&create_info).unwrap();
    }

    let mut last_frame_start = Instant::now();
    let mut frame_count_time: (u32, f32) = (0, 0.0);

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::NewEvents(_) => {}
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                info!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::MainEventsCleared => {
                let last_frame_time = last_frame_start.elapsed();
                last_frame_start = Instant::now();

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
}
