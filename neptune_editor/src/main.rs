#[macro_use]
extern crate log;

use neptune_graphics::{
    AddressMode, AppInfo, BufferDescription, BufferUsage, FilterMode, SamplerDescription,
    TextureDescription, TextureFormat, TextureUsage,
};
use std::default::Default;
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

    let device = {
        let new_instance = neptune_graphics::create_vulkan_instance(
            &AppInfo::new("Neptune Engine", [0, 0, 1, 0]),
            &AppInfo::new(APP_NAME, [0, 0, 1, 0]),
        );

        let surface = new_instance.create_surface("Main Surface", &window).ok();

        let selected_device = new_instance
            .select_and_create_device(surface.as_ref(), |index, device_info| {
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

        selected_device.create(&create_info).unwrap()
    };

    let buffer = device
        .create_buffer(
            "Test Buffer",
            &BufferDescription {
                size: 4096,
                usage: BufferUsage::VERTEX,
            },
        )
        .unwrap();

    let sampler = device
        .create_sampler(
            "Linear Sampler",
            &SamplerDescription {
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                address_mode_w: AddressMode::Repeat,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mip_filter: FilterMode::Linear,
                ..Default::default()
            },
        )
        .unwrap();

    let texture = device
        .create_texture(
            "Test Image",
            &TextureDescription {
                size: [4096, 4096],
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TRANSFER_SRC,
                sampler: None,
            },
        )
        .unwrap();

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
                device.render_frame(|_render_graph_builder| {}).unwrap();

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
