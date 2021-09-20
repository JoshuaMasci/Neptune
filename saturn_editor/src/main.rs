use saturn_rendering;

use winit::platform::run_return::EventLoopExtRunReturn;
pub use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

fn main() {
    let app = saturn_rendering::AppInfo {
        name: "Saturn Editor".to_string(),
        version: saturn_rendering::AppVersion::new(0, 0, 0),
    };

    let mut event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title(app.name.as_str())
        .with_resizable(true)
        .with_maximized(true)
        .build(&event_loop)
        .unwrap();

    let mut vulkan_instance = saturn_rendering::Instance::new(&app, &window);
    let mut _vulkan_device = vulkan_instance.create_device(0);

    event_loop.run_return(move |event, _, control_flow| {
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
                //Run stuff here
            }
            Event::RedrawRequested(_) => {}
            _ => (),
        }
    });

    println!("Application Has closed");
}

/* BACKEND CODE IDEAS
 * let instance = ...;
 * let surface = instance.create_surface(window);
 *
 * Setup stuff
 * let devices = instance.get_all_devices();
 * let device = instance.create_device(CHOSEN_DEVICE);
 * let swapchain = device.create_swapchain(surface).expect("Failed to create swapchain for device");
 *
 * Creates a texture and fills it with data
 * let texture_id = device.create_texture(tex_info, DATA);
 *
 * let blit_render_pass = RenderPass::new(
 * {
 *      let present_image_id = get
 *
 *      Returns the resource ids used by this pass
 *      Needed for sync and scheduling
 *      TODO: specify Image Layout transitions
 *      return vec![ Read(texture_id), Write(present_image_id)];
 * },
 * {
 *      let input_texture: SampleImage = Function Params[0];
 *      let present_image: SampleImage = Function Params[1];
 *
 *      Blit Images
 * }
 * );
 *
 * device.render(&[blit_render_pass]);
 */
