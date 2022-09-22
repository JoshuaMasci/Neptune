mod imgui_layer;
mod scene_layer;

use neptune_core::log::{debug, error, info, trace, warn};
use std::time::Instant;
use winit::platform::run_return::EventLoopExtRunReturn;
pub use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

fn main() {
    neptune_core::setup_logger().expect("Failed to init logger");

    test_render_api();

    let mut event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Neptune Editor")
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();

    window.set_maximized(true);

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
                // device.render_frame(|render_graph_builder| {
                // });
            }
            Event::RedrawRequested(_window_id) => {}
            _ => {}
        }
    });
    info!("Exiting Main Loop!");
}

use neptune_graphics::{
    AddressMode, Attachment, BorderColor, BufferUsage, DeviceTrait, DeviceType, FilterMode,
    IndexSize, InstanceTrait, PipelineState, RasterPass, SamplerCreateInfo, TextureCreateInfo,
    TextureFormat, TextureUsage,
};

//TODO: verify that this works
fn to_bytes_unsafe<T>(data: &[T]) -> &[u8] {
    let ptr = data.as_ptr();
    let ptr_size = std::mem::size_of::<T>() * data.len();
    unsafe { &*std::ptr::slice_from_raw_parts(ptr as *const u8, ptr_size) }
}

fn test_render_api() {
    let mut instance = neptune_graphics::create_instance();

    let surface = instance.create_surface().unwrap();

    info!("Available Devices: ");
    let mut device = instance
        .select_and_create_device(Some(&surface), |device_info| {
            println!("\t\t{:?}", device_info);
            match device_info.device_type {
                DeviceType::Integrated => 50,
                DeviceType::Discrete => 100,
                DeviceType::Unknown => 0,
            }
        })
        .unwrap();
    info!("Selected Device: {:?}", device.device_info);

    let triangle_vertex_buffer = device
        .create_static_buffer(
            BufferUsage::VERTEX,
            to_bytes_unsafe(&[
                0.0f32, 0.5f32, 0.0f32, -0.5f32, -0.5f32, 0.0f32, 0.5f32, -0.5f32, 0.0f32,
            ]),
        )
        .unwrap();

    let triangle_index_buffer = device
        .create_static_buffer(BufferUsage::INDEX, to_bytes_unsafe(&[0u32, 1u32, 2u32]))
        .unwrap();

    let purple_texture = device
        .create_static_texture(
            &TextureCreateInfo {
                format: TextureFormat::Rgba8Unorm,
                size: [1; 2],
                usage: TextureUsage::SAMPLED,
                mip_levels: 1,
                sample_count: 1,
            },
            &[75, 0, 130, 255],
        )
        .unwrap();

    let linear_sampler = device
        .create_sampler(&SamplerCreateInfo {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mip_filter: FilterMode::Linear,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            min_lod: 0.0,
            max_lod: 0.0,
            max_anisotropy: None,
            boarder_color: BorderColor::TransparentBlack,
        })
        .unwrap();

    let basic_vertex_shader = device.create_vertex_shader(&[0, 1, 2]).unwrap();
    let basic_fragment_shader = device.create_fragment_shader(&[0, 1, 2]).unwrap();

    device.render_frame(|render_graph_builder, swapchain_texture| {
        let swapchain_texture = swapchain_texture.unwrap();

        let mut raster_pass = RasterPass::new("Triangle Pass");
        raster_pass.color_attachment(Attachment::new_with_clear(
            &swapchain_texture,
            &[0.0, 0.0, 0.0, 1.0],
        ));

        let default_pipeline_state = PipelineState::default();
        raster_pass.pipeline(
            &basic_vertex_shader,
            Some(&basic_fragment_shader),
            &default_pipeline_state,
            &[],
            |raster_command_buffer| {
                raster_command_buffer.bind_vertex_buffer(&triangle_vertex_buffer, 0);
                raster_command_buffer.bind_index_buffer(&triangle_index_buffer, 0, IndexSize::U32);
                raster_command_buffer.bind_sampler(0, &linear_sampler);
                raster_command_buffer.bind_sampled_texture(1, &purple_texture);
                raster_command_buffer.draw_indexed(0..3, 0, 0..1);
            },
        );

        render_graph_builder.add_raster_pass(raster_pass);
    });
}
