use neptune_core::log::{debug, error, info, trace, warn};
use neptune_vulkan::ash::vk::BufferUsageFlags;
use neptune_vulkan::MemoryType;
use std::time::Instant;
use winit::platform::run_return::EventLoopExtRunReturn;
pub use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

const APP_NAME: &str = "Neptune Editor";

fn main() {
    neptune_core::setup_logger().expect("Failed to init logger");

    let mut event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title(APP_NAME)
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();
    window.set_maximized(true);

    let mut instance =
        neptune_vulkan::Instance::new(APP_NAME).expect("Failed to create vulkan instance");

    let surface = instance.create_surface(&window);

    info!("Available Devices: ");
    let device = instance
        .select_and_create_device(None, |device_info| {
            println!("\t\t{:?}", device_info);
            match device_info.device_type {
                neptune_vulkan::DeviceType::Integrated => 50,
                neptune_vulkan::DeviceType::Discrete => 100,
                neptune_vulkan::DeviceType::Unknown => 0,
            }
        })
        .unwrap();
    info!("Selected Device: {:?}", device.info());

    {
        let buffer = device
            .create_buffer(
                "Some Buffer Name",
                &neptune_vulkan::ash::vk::BufferCreateInfo::builder()
                    .size(2 ^ 16)
                    .usage(BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::STORAGE_BUFFER)
                    .build(),
                MemoryType::GpuOnly,
            )
            .expect("Failed to create buffer!");
    }

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
                println!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::MainEventsCleared => {}
            Event::RedrawRequested(_window_id) => {}
            event => {}
        }
    });
    info!("Exiting Main Loop!");
}
