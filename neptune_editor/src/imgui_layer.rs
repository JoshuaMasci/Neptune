use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use imgui::{Context, DrawCmd, DrawCmdParams, DrawData};
use imgui_winit_support::WinitPlatform;
use neptune_core::log::warn;

struct ImguiContext {
    context: Context,
    needs_render: bool,
}

impl ImguiContext {
    fn render(&mut self) -> &DrawData {
        self.needs_render = false;
        self.context.render()
    }
}

pub struct ImguiLayer {
    imgui_context: Rc<RefCell<ImguiContext>>,
    winit_platform: WinitPlatform,
    // vertex_module: Rc<vulkan::ShaderModule>,
    // fragment_module: Rc<vulkan::ShaderModule>,
    // texture_atlas: Rc<Resource<vulkan::Texture>>,
}

struct ImguiDraw {
    display_pos: [f32; 2],
    display_size: [f32; 2],
    framebuffer_scale: [f32; 2],
    offsets: Vec<(i32, u32)>,
    draw_lists: Vec<Vec<imgui::DrawCmd>>,
}

unsafe fn to_byte_vector<S>(src: Vec<S>) -> Vec<u8> {
    // Ensure the original vector is not dropped.
    let mut src = std::mem::ManuallyDrop::new(src);
    let len = std::mem::size_of::<S>() * src.len();
    let capacity = std::mem::size_of::<S>() * src.capacity();

    Vec::from_raw_parts(src.as_mut_ptr() as *mut u8, len, capacity)
}

impl ImguiLayer {
    pub fn new() -> Self {
        let mut imgui_context = Context::create();
        let winit_platform = WinitPlatform::init(&mut imgui_context);

        // let texture_atlas_data = imgui_context.fonts().build_alpha8_texture();
        // let texture_atlas = Rc::new(device.create_texture_with_data(
        //     TextureDescription {
        //         format: TextureFormat::R8Unorm,
        //         size: TextureDimensions::D2(texture_atlas_data.width, texture_atlas_data.height),
        //         usage: TextureUsages::SAMPLED | TextureUsages::TRANSFER_DST,
        //         memory_type: MemoryType::GpuOnly,
        //     },
        //     texture_atlas_data.data,
        // ));
        //
        // let vertex_module = Rc::new(device.create_shader_module(crate::shader::IMGUI_VERT));
        // let fragment_module = Rc::new(device.create_shader_module(crate::shader::IMGUI_FRAG));

        let imgui_context = Rc::new(RefCell::new(ImguiContext {
            context: imgui_context,
            needs_render: false,
        }));

        Self {
            imgui_context,
            winit_platform,
        }
    }

    pub fn handle_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::Event<()>,
    ) {
        self.winit_platform.handle_event(
            (*self.imgui_context).borrow_mut().context.io_mut(),
            window,
            event,
        );
    }

    pub(crate) fn update_time(&mut self, last_frame_time: Duration) {
        (*self.imgui_context)
            .borrow_mut()
            .context
            .io_mut()
            .update_delta_time(last_frame_time);
    }

    pub fn build_frame(
        &mut self,
        window: &winit::window::Window,
        callback: impl FnOnce(&mut imgui::Ui),
    ) {
        let mut imgui_context = (*self.imgui_context).borrow_mut();

        //If the last frame didn't render call render to clear the frame data
        if imgui_context.needs_render {
            let _ = imgui_context.render();
        }

        self.winit_platform
            .prepare_frame(imgui_context.context.io_mut(), window)
            .expect("Failed to prepare frame");
        let ui = imgui_context.context.frame();

        callback(ui);

        self.winit_platform.prepare_render(ui, window);
        imgui_context.needs_render = true;
    }
}

impl Drop for ImguiLayer {
    fn drop(&mut self) {
        println!("ImguiLayer::Drop");
    }
}

// fn collect_mesh_buffers(
//     draw_data: &DrawData,
// ) -> (Vec<imgui::DrawVert>, Vec<imgui::DrawIdx>, Vec<(i32, u32)>) {
//     let mut vertices = Vec::with_capacity(draw_data.total_vtx_count as usize);
//     let mut indices = Vec::with_capacity(draw_data.total_idx_count as usize);
//     let mut offsets = Vec::new();
//     for draw_list in draw_data.draw_lists() {
//         let vertex_buffer = draw_list.vtx_buffer();
//         let index_buffer = draw_list.idx_buffer();
//         offsets.push((vertices.len() as i32, indices.len() as u32));
//         vertices.extend_from_slice(vertex_buffer);
//         indices.extend_from_slice(index_buffer);
//     }
//     (vertices, indices, offsets)
// }
