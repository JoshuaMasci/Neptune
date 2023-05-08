use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use imgui::{Context, DrawCmd, DrawCmdParams, DrawData};
use imgui_winit_support::WinitPlatform;

use neptune_graphics::{
    vulkan, BufferDescription, BufferUsages, ColorAttachment, IndexSize, MemoryType, PipelineState,
    RasterPassBuilder, RenderGraphBuilder, TextureDescription, TextureDimensions, TextureFormat,
    UploadData, VertexElement,
};
use neptune_graphics::{Resource, TextureUsages};

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

    vertex_module: Rc<vulkan::ShaderModule>,
    fragment_module: Rc<vulkan::ShaderModule>,
    texture_atlas: Rc<Resource<vulkan::Texture>>,
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
    pub fn new(device: &mut vulkan::Device) -> Self {
        let mut imgui_context = Context::create();
        let winit_platform = WinitPlatform::init(&mut imgui_context);

        let texture_atlas_data = imgui_context.fonts().build_alpha8_texture();
        let texture_atlas = Rc::new(device.create_texture_with_data(
            TextureDescription {
                format: TextureFormat::R8Unorm,
                size: TextureDimensions::D2(texture_atlas_data.width, texture_atlas_data.height),
                usage: TextureUsages::SAMPLED | TextureUsages::TRANSFER_DST,
                memory_type: MemoryType::GpuOnly,
            },
            texture_atlas_data.data,
        ));

        let vertex_module = Rc::new(device.create_shader_module(crate::shader::IMGUI_VERT));
        let fragment_module = Rc::new(device.create_shader_module(crate::shader::IMGUI_FRAG));

        let imgui_context = Rc::new(RefCell::new(ImguiContext {
            context: imgui_context,
            needs_render: false,
        }));

        Self {
            imgui_context,
            winit_platform,

            vertex_module,
            fragment_module,
            texture_atlas,
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

    pub fn render_frame(
        &mut self,
        render_graph: &mut RenderGraphBuilder,
        render_target: neptune_graphics::TextureId,
    ) {
        let mut imgui_context = (*self.imgui_context).borrow_mut();

        let texture_atlas = render_graph.import_texture(self.texture_atlas.clone());

        let draw_data = imgui_context.render();

        let (vertices, indices, offsets) = collect_mesh_buffers(draw_data);

        //Nothing to render
        if offsets.is_empty() {
            return;
        }

        let draw_data = ImguiDraw {
            display_pos: draw_data.display_pos,
            display_size: draw_data.display_size,
            framebuffer_scale: draw_data.framebuffer_scale,
            offsets,
            draw_lists: draw_data
                .draw_lists()
                .map(|draw_list| draw_list.commands().collect())
                .collect(),
        };

        //Transmute between vector types
        let vertices: Vec<u8> = unsafe { to_byte_vector(vertices) };
        let indices: Vec<u8> = unsafe { to_byte_vector(indices) };

        let vertex_buffer = render_graph.create_buffer(BufferDescription {
            size: vertices.len(),
            usage: BufferUsages::VERTEX | BufferUsages::TRANSFER_DST,
            memory_type: MemoryType::GpuOnly,
        });

        let index_buffer = render_graph.create_buffer(BufferDescription {
            size: indices.len(),
            usage: BufferUsages::INDEX | BufferUsages::TRANSFER_DST,
            memory_type: MemoryType::GpuOnly,
        });

        render_graph.add_buffer_upload_pass(vertex_buffer, 0, UploadData::U8(vertices));
        render_graph.add_buffer_upload_pass(index_buffer, 0, UploadData::U8(indices));

        let mut raster_pass = RasterPassBuilder::new("ImguiLayer");
        raster_pass.vertex_buffer(vertex_buffer);
        raster_pass.index_buffer(index_buffer);
        raster_pass.shader_read_texture(texture_atlas);
        raster_pass.attachments(
            &[ColorAttachment {
                id: render_target,
                clear: None,
            }],
            None,
        );
        raster_pass.pipeline(
            self.vertex_module.clone(),
            Some(self.fragment_module.clone()),
            vec![
                VertexElement::Float2,
                VertexElement::Float2,
                VertexElement::Byte4,
            ],
            PipelineState::alpha_blending_basic(),
            move |command_buffer| {
                command_buffer.push_texture((std::mem::size_of::<f32>() * 4) as u32, texture_atlas);
                command_buffer.bind_vertex_buffers(vertex_buffer, 0);
                command_buffer.bind_index_buffer(index_buffer, 0, IndexSize::U16);

                let framebuffer_width = draw_data.framebuffer_scale[0] * draw_data.display_size[0];
                let framebuffer_height = draw_data.framebuffer_scale[1] * draw_data.display_size[1];

                //Push data
                let mut push_data = [0f32; 4];
                //Scale
                push_data[0] = 2.0 / draw_data.display_size[0];
                push_data[1] = 2.0 / draw_data.display_size[1];
                //Translate
                push_data[2] = -1.0 - (draw_data.display_pos[0] * push_data[0]);
                push_data[3] = -1.0 - (draw_data.display_pos[1] * push_data[1]);

                command_buffer.push_floats(0, &push_data);

                let clip_offset = draw_data.display_pos;
                let clip_scale = draw_data.framebuffer_scale;

                for (draw_list, (vertex_offset, index_offset)) in
                    draw_data.draw_lists.iter().zip(draw_data.offsets.iter())
                {
                    for command in draw_list.iter() {
                        match command {
                            DrawCmd::Elements {
                                count,
                                cmd_params:
                                    DrawCmdParams {
                                        clip_rect,
                                        texture_id,
                                        vtx_offset,
                                        idx_offset,
                                    },
                            } => {
                                let mut clip_rect: [f32; 4] = [
                                    (clip_rect[0] - clip_offset[0]) * clip_scale[0],
                                    (clip_rect[1] - clip_offset[1]) * clip_scale[1],
                                    (clip_rect[2] - clip_offset[0]) * clip_scale[0],
                                    (clip_rect[3] - clip_offset[1]) * clip_scale[1],
                                ];

                                if (clip_rect[0] < framebuffer_width)
                                    && (clip_rect[1] < framebuffer_height)
                                    && (clip_rect[2] >= 0.0)
                                    && (clip_rect[3] >= 0.0)
                                {
                                    clip_rect[0] = clip_rect[0].max(0.0);
                                    clip_rect[1] = clip_rect[1].max(0.0);

                                    command_buffer.set_scissor(
                                        [clip_rect[0] as i32, clip_rect[1] as i32],
                                        [
                                            (clip_rect[2] - clip_rect[0]) as u32,
                                            (clip_rect[3] - clip_rect[1]) as u32,
                                        ],
                                    );

                                    command_buffer.draw_indexed(
                                        *count as u32,
                                        index_offset + *idx_offset as u32,
                                        vertex_offset + *vtx_offset as i32,
                                        1,
                                        0,
                                    )
                                }
                            }
                            DrawCmd::ResetRenderState => {}
                            DrawCmd::RawCallback { .. } => {}
                        }
                    }
                }
            },
        );
        render_graph.add_raster_pass(raster_pass);
    }
}

impl Drop for ImguiLayer {
    fn drop(&mut self) {
        println!("ImguiLayer::Drop");
    }
}

fn collect_mesh_buffers(
    draw_data: &DrawData,
) -> (Vec<imgui::DrawVert>, Vec<imgui::DrawIdx>, Vec<(i32, u32)>) {
    let mut vertices = Vec::with_capacity(draw_data.total_vtx_count as usize);
    let mut indices = Vec::with_capacity(draw_data.total_idx_count as usize);
    let mut offsets = Vec::new();
    for draw_list in draw_data.draw_lists() {
        let vertex_buffer = draw_list.vtx_buffer();
        let index_buffer = draw_list.idx_buffer();
        offsets.push((vertices.len() as i32, indices.len() as u32));
        vertices.extend_from_slice(vertex_buffer);
        indices.extend_from_slice(index_buffer);
    }
    (vertices, indices, offsets)
}
