#[macro_use]
extern crate log;

#[macro_use]
extern crate neptune_vulkan_macro;

use neptune_vulkan::ash::vk::{
    BufferUsageFlags, Extent3D, Format, ImageType, ImageUsageFlags, SampleCountFlags,
};
use neptune_vulkan::MemoryLocation;
use std::sync::Arc;
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

    let buffer = device
        .create_buffer(
            "Test Buffer",
            &neptune_vulkan::ash::vk::BufferCreateInfo::builder()
                .size(2 ^ 16)
                .usage(BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::UNIFORM_BUFFER)
                .build(),
            MemoryLocation::CpuToGpu,
        )
        .expect("Failed to create buffer!");
    assert!(buffer.fill(&[0u32; 16]).is_ok(), "Buffer should be mapped");

    let image = device
        .create_image(
            "Test Image",
            &neptune_vulkan::ash::vk::ImageCreateInfo::builder()
                .format(Format::R8G8B8A8_UNORM)
                .extent(Extent3D {
                    width: 1920,
                    height: 1080,
                    depth: 1,
                })
                .image_type(ImageType::TYPE_2D)
                .usage(ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::SAMPLED)
                .samples(SampleCountFlags::TYPE_1)
                .mip_levels(1)
                .array_layers(1)
                .build(),
            MemoryLocation::GpuOnly,
        )
        .unwrap();

    let image_view = device
        .create_image_view(
            "Test Image View",
            image.clone(),
            &image.get_full_image_view_create_info().build(),
        )
        .unwrap();

    let basic_descriptor_pool = device
        .create_descriptor_pool::<TestDescriptorSet>(1)
        .unwrap();
    let descriptor_set = basic_descriptor_pool
        .create_set(
            "",
            TestDescriptorSet {
                some_uniform_buffer: buffer,
            },
        )
        .unwrap();

    drop(device);

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
            Event::MainEventsCleared => {}
            Event::RedrawRequested(_window_id) => {}
            event => {}
        }
    });
    info!("Exiting Main Loop!");
}

#[derive(DescriptorSet)]
struct TestDescriptorSet {
    #[binding(uniform_buffer)]
    some_uniform_buffer: Arc<neptune_vulkan::Buffer>,
}
