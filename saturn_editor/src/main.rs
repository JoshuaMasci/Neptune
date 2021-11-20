mod transform;
mod world;

use saturn_rendering;

use crate::world::World;
use saturn_rendering::command_buffer::CommandBuffer;
use saturn_rendering::render_task::ResourceAccess;
use saturn_rendering::render_task::ResourceAccess::{ReadImage, WriteImage};
use saturn_rendering::vk::ClearColorValue;
use saturn_rendering::ImageId;
use std::ops::Deref;
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

    // let mut vulkan_instance = saturn_rendering::Instance::new(&app, &window);
    // let mut vulkan_device = vulkan_instance.create_device(0);

    let mut world = World::new();

    // let mut image = vulkan_device.create_image(
    //     saturn_rendering::vk::Format::R8G8B8A8_UNORM,
    //     saturn_rendering::vk::Extent2D::builder()
    //         .width(1920)
    //         .height(1080)
    //         .build(),
    //     saturn_rendering::vk::ImageUsageFlags::TRANSFER_SRC
    //         | saturn_rendering::vk::ImageUsageFlags::STORAGE,
    //     saturn_rendering::gpu_allocator::MemoryLocation::GpuOnly,
    // );

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
                // let clear_image_color = |command_buffer: &mut CommandBuffer| {
                //     command_buffer.clear_color_image(&mut image, &[0.0, 0.5, 8.0, 1.0]);
                // };
                //
                // vulkan_device.render(&[clear_image_color]);
                vulkan_device.draw();
            }
            Event::RedrawRequested(_) => {}
            _ => (),
        }
    });
}

// struct ClearTask {
//     image: ImageId,
//     clear_color: [f32; 4],
// }
//
// impl saturn_rendering::render_task::RenderTask for ClearTask {
//     fn get_resources(&self) -> Vec<ResourceAccess> {
//         return vec![WriteImage(self.image)];
//     }
//
//     fn build_command(&self, frame_index: u32, command_buffer: &mut CommandBuffer) {
//         println!("Clearing {:?} to {:?}", self.image, self.clear_color);
//         //command_buffer.clear_color_image(image, &self.clear_color);
//     }
// }
//
// struct BlitTask {
//     src_image: ImageId,
//     dst_image: ImageId,
// }
//
// impl saturn_rendering::render_task::RenderTask for BlitTask {
//     fn get_resources(&self) -> Vec<ResourceAccess> {
//         return vec![ReadImage(self.src_image), WriteImage(self.dst_image)];
//     }
//
//     fn build_command(&self, frame_index: u32, command_buffer: &mut CommandBuffer) {
//         println!("Blit-ing {:?} to {:?}", self.src_image, self.dst_image);
//     }
// }

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
 * device.render(swapchain, &[blit_render_pass]);
 */
