#[macro_use]
extern crate log;

use neptune_vulkan::ash::vk::Format;
use neptune_vulkan::{
    AddressMode, BufferUsage, CompositeAlphaMode, FilterMode, PresentMode, SamplerCreateInfo,
    TextureUsage,
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

    let mut instance =
        neptune_vulkan::Instance::new(APP_NAME).expect("Failed to create vulkan instance");

    let surface = instance
        .create_surface(&window)
        .expect("Failed to create vulkan surface");

    info!("Available Devices: ");
    let device = instance
        .select_and_create_device(Some(&surface), |device_info| {
            println!("\t\t{:?}", device_info);
            match device_info.device_type {
                neptune_vulkan::DeviceType::Integrated => 50,
                neptune_vulkan::DeviceType::Discrete => 100,
                neptune_vulkan::DeviceType::Unknown => 0,
            }
        })
        .unwrap();
    info!("Selected Device: {:?}", device.info());

    let swapchain = device
        .create_swapchain(
            &surface,
            neptune_vulkan::SwapchainConfig {
                format: Format::B8G8R8A8_UNORM,
                present_mode: PresentMode::Fifo,
                usage: TextureUsage::ATTACHMENT,
                composite_alpha: CompositeAlphaMode::Auto,
            },
        )
        .unwrap();

    let buffer = device
        .create_buffer_with_data(
            "Test Buffer",
            BufferUsage::VERTEX | BufferUsage::STORAGE | BufferUsage::UNIFORM,
            &[0u8; 16],
        )
        .unwrap();

    let sampler = device
        .create_sampler(
            "Test Sampler",
            &SamplerCreateInfo {
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                address_mode_w: AddressMode::Repeat,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                ..Default::default()
            },
        )
        .unwrap();

    let texture = device
        .create_texture_with_data(
            "Test Texture",
            TextureUsage::STORAGE,
            Format::R8G8B8A8_UNORM,
            [1; 2],
            Some(&sampler),
            &[93, 63, 211, 255],
        )
        .unwrap();

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
                info!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::MainEventsCleared => {
                device.render_frame();
            }
            Event::RedrawRequested(_window_id) => {}
            event => {}
        }
    });
    info!("Exiting Main Loop!");
}
