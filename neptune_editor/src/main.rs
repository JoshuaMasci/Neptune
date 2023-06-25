#[macro_use]
extern crate log;

use neptune_vulkan::vk;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::sync::Arc;
use std::time::Instant;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::{
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

    //API Test
    neptune_vulkan::test_api();

    {
        let _instance = neptune_vulkan::AshInstance::new(
            &neptune_vulkan::AppInfo::new("Neptune Engine", [0, 0, 1, 0]),
            &neptune_vulkan::AppInfo::new("Neptune Editor", [0, 0, 1, 0]),
            true,
            Some(event_loop.raw_display_handle()),
        )
        .map(Arc::new)
        .unwrap();
        let surface = _instance
            .crate_surface(window.raw_display_handle(), window.raw_window_handle())
            .unwrap();

        let physical_device = unsafe { _instance.core.enumerate_physical_devices() }.unwrap()[0];
        let _device = neptune_vulkan::AshDevice::new(_instance, physical_device, &[0])
            .map(Arc::new)
            .unwrap();
        let _swapchain = neptune_vulkan::AshSwapchain::new(
            _device.clone(),
            surface,
            neptune_vulkan::AshSwapchainSettings {
                image_count: 3,
                format: vk::SurfaceFormatKHR {
                    format: vk::Format::B8G8R8A8_UNORM,
                    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
                },
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
                present_mode: vk::PresentModeKHR::FIFO,
            },
        )
        .unwrap();
        let _resource_manager = neptune_vulkan::PersistentResourceManager::new(3);

        drop(_swapchain);
        unsafe {
            _device.instance.surface.destroy_surface(surface, None);
        }
    }

    let mut last_frame_start = Instant::now();
    let mut frame_count_time: (u32, f32) = (0, 0.0);

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
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
