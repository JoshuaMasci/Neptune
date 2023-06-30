#[macro_use]
extern crate log;

use neptune_vulkan::vk;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::collections::HashMap;
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
    //window.set_maximized(true);

    //Vulkan Start
    let instance = neptune_vulkan::AshInstance::new(
        &neptune_vulkan::AppInfo::new("Neptune Engine", [0, 0, 1, 0]),
        &neptune_vulkan::AppInfo::new("Neptune Editor", [0, 0, 1, 0]),
        true,
        Some(event_loop.raw_display_handle()),
    )
    .map(Arc::new)
    .unwrap();
    let surface = instance
        .crate_surface(window.raw_display_handle(), window.raw_window_handle())
        .unwrap();

    let physical_device = unsafe { instance.core.enumerate_physical_devices() }.unwrap()[0];
    let device = neptune_vulkan::AshDevice::new(instance, physical_device, &[0])
        .map(Arc::new)
        .unwrap();
    let mut swapchain_manager = neptune_vulkan::SwapchainManager::default();
    swapchain_manager.add_swapchain(
        neptune_vulkan::AshSwapchain::new(
            device.clone(),
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
        .unwrap(),
    );
    let mut resource_manager = neptune_vulkan::PersistentResourceManager::new(3);
    let mut graph_executor =
        neptune_vulkan::BasicRenderGraphExecutor::new(device.clone(), 0).unwrap();

    let mut render_graph = neptune_vulkan::RenderGraph::default();
    let swapchain_image = render_graph.acquire_swapchain_image(surface);

    let mut image_usages = HashMap::new();
    image_usages.insert(
        swapchain_image,
        neptune_vulkan::ImageAccess {
            write: true,
            stage: vk::PipelineStageFlags2::FRAGMENT_SHADER,
            access: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            layout: vk::ImageLayout::ATTACHMENT_OPTIMAL,
        },
    );
    let basic_render_pass = neptune_vulkan::RenderPass {
        name: String::from("Basic Render Pass"),
        queue: vk::Queue::null(), //TODO: this
        buffer_usages: Default::default(),
        image_usages,
        framebuffer: Some(neptune_vulkan::Framebuffer {
            color_attachments: vec![neptune_vulkan::ColorAttachment::new_clear(
                swapchain_image,
                [0.0, 0.0, 0.0, 0.0],
            )],
            depth_stencil_attachment: None,
            input_attachments: vec![],
        }),
        build_cmd_fn: None,
    };
    render_graph.add_pass(basic_render_pass);

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

                //Vulkan Frame
                graph_executor
                    .execute_graph(
                        &render_graph,
                        &mut neptune_vulkan::RenderGraphResources {
                            persistent: &mut resource_manager,
                            swapchain: &mut swapchain_manager,
                        },
                    )
                    .expect("Failed to execute graph");
            }
            Event::RedrawRequested(_window_id) => {}
            _ => {}
        }
    });

    //Cleanup
    unsafe {
        device.instance.surface.destroy_surface(surface, None);
    }

    info!("Exiting Main Loop!");
}
