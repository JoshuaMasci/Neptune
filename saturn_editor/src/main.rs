use saturn_rendering::*;

fn main() {
    let app = graphics::AppInfo {
        name: "Saturn Editor".to_string(),
        version: graphics::AppVersion::new(0, 0, 0),
    };

    let (mut vulkan_graphics, (event_loop, _window)) = graphics::Graphics::new(&app);

    let id = vulkan_graphics.create_storage_buffer(
        vk::BufferUsageFlags::empty(),
        gpu_allocator::MemoryLocation::GpuOnly,
        512,
    );
    vulkan_graphics.destroy_storage_buffer(id);

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
