use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use imgui::Context;
use imgui_winit_support::WinitPlatform;

use neptune_graphics::{vulkan, MemoryType, TextureDescription, TextureDimensions, TextureFormat};
use neptune_graphics::{Resource, TextureUsages};

pub struct ImguiLayer {
    imgui_context: Rc<RefCell<Context>>,
    winit_platform: WinitPlatform,

    needs_render: bool,

    shader_modules: Rc<(vulkan::ShaderModule, vulkan::ShaderModule)>,
    texture_atlas: Rc<Resource<vulkan::Texture>>,
    texture_atlas_data: Option<Vec<u8>>,
}

impl ImguiLayer {
    pub fn new(device: &mut vulkan::Device) -> Self {
        let mut imgui_context = Context::create();
        let winit_platform = WinitPlatform::init(&mut imgui_context);

        let texture_atlas_data = imgui_context.fonts().build_alpha8_texture();
        let texture_atlas = Rc::new(device.create_texture(TextureDescription {
            format: TextureFormat::R8Unorm,
            size: TextureDimensions::D2(texture_atlas_data.width, texture_atlas_data.height),
            usage: TextureUsages::SAMPLED | TextureUsages::TRANSFER_DST,
            memory_type: MemoryType::GpuOnly,
        }));
        let texture_atlas_data = Some(texture_atlas_data.data.to_vec());

        let shader_modules = Rc::new((
            device.create_shader_module(crate::shader::IMGUI_VERT),
            device.create_shader_module(crate::shader::IMGUI_FRAG),
        ));

        let imgui_context = Rc::new(RefCell::new(imgui_context));
        Self {
            imgui_context,
            winit_platform,

            needs_render: false,

            shader_modules,
            texture_atlas,
            texture_atlas_data,
        }
    }

    pub fn handle_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::Event<()>,
    ) {
        self.winit_platform
            .handle_event(self.imgui_context.borrow_mut().io_mut(), window, event);
    }

    pub(crate) fn update_time(&self, last_frame_time: Duration) {
        self.imgui_context
            .borrow_mut()
            .io_mut()
            .update_delta_time(last_frame_time);
    }

    pub fn build_frame(
        &mut self,
        window: &winit::window::Window,
        callback: impl FnOnce(&mut imgui::Ui),
    ) {
        let mut imgui_context = self.imgui_context.borrow_mut();

        //If the last frame didn't render call render to clear the frame data
        if self.needs_render {
            let _ = imgui_context.render();
        }

        self.winit_platform
            .prepare_frame(imgui_context.io_mut(), window)
            .expect("Failed to prepare frame");
        let ui = imgui_context.frame();

        callback(ui);

        self.winit_platform.prepare_render(ui, window);
        self.needs_render = true;
    }
}

impl Drop for ImguiLayer {
    fn drop(&mut self) {
        println!("ImguiLayer::Drop");
    }
}
