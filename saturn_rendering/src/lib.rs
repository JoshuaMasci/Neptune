//Old stuff probably won't use
mod buffer;
mod command_buffer;
mod descriptor_set;
mod device;
mod id_pool;
mod image;
mod instance;
mod pipeline;
mod render_task;
mod swapchain;
use ash::*;
use gpu_allocator;
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct BufferId(u32);
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct ImageId(u32);
const SwapchainImageId: ImageId = ImageId(u32::MAX);

pub struct Instance {} //vk::Instance

impl Instance {
    pub fn enumerate_adapters() -> Vec<Adapter> {
        Vec::new()
    }

    //TODO: make generic
    pub fn create_surface(&self, window: &winit::window::Window) -> Surface {
        Surface {}
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        todo!()
    }
}

pub struct Surface {} //vk::SurfaceKHR

pub struct Adapter {} //vk::PhysicalDevice

pub struct Device {}

impl Device {
    pub fn create_swapchain(&self, surface: &Surface) {}
}

pub struct Buffer {}
pub struct Image {}
pub struct RenderPipeline {}
pub struct ComputePipeline {}
