use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::cell::RefCell;
use std::ffi::CString;
use std::rc::Rc;
use std::time::Instant;

use crate::render_backend::RenderDevice;
use crate::render_graph::{render_graph, ImageHandle};
use crate::vulkan::framebuffer::FrameBufferSet;
use crate::vulkan::{Buffer, BufferDescription};
use crate::vulkan::{Image, ImageDescription};
use ash::vk;
use ash::vk::Offset2D;
use gpu_allocator::MemoryLocation;
use imgui::{DrawCmd, DrawCmdParams, DrawData, DrawIdx, DrawVert};

pub struct ImguiLayer {
    imgui_context: Rc<RefCell<imgui::Context>>,
    winit_platform: WinitPlatform,
    device: RenderDevice,

    texture_atlas: Image,
    texture_sampler: vk::Sampler,

    descriptor_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,

    texture_atlas_data: Option<Vec<u8>>,
}

impl ImguiLayer {
    pub fn new(window: &winit::window::Window, device: RenderDevice) -> Self {
        let mut imgui_context = imgui::Context::create();
        let mut winit_platform = WinitPlatform::init(&mut imgui_context);
        winit_platform.attach_window(imgui_context.io_mut(), window, HiDpiMode::Default);

        //Config imgui
        imgui_context.io_mut().config_flags |= imgui::ConfigFlags::DOCKING_ENABLE;
        //imgui_context.set_ini_filename(None);
        imgui_context.set_renderer_name(Some(String::from("Neptune Renderer")));

        let image_data = imgui_context.fonts().build_alpha8_texture();

        let mut texture_atlas = Image::new(
            &device,
            ImageDescription {
                format: vk::Format::R8_UNORM,
                size: [image_data.width, image_data.height],
                usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                memory_location: MemoryLocation::GpuOnly,
            },
        );
        texture_atlas.create_image_view();
        let image_data = image_data.data.to_vec();

        let texture_sampler = unsafe {
            device.base.create_sampler(
                &vk::SamplerCreateInfo::builder()
                    .mag_filter(vk::Filter::LINEAR)
                    .min_filter(vk::Filter::LINEAR)
                    .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                    .address_mode_u(vk::SamplerAddressMode::REPEAT)
                    .address_mode_v(vk::SamplerAddressMode::REPEAT)
                    .address_mode_w(vk::SamplerAddressMode::REPEAT)
                    .max_anisotropy(1.0)
                    .min_lod(-1000.0)
                    .max_lod(1000.0),
                None,
            )
        }
        .expect("Failed to create sampler");

        //TODO: replace Push Descriptors once images are supported
        let descriptor_layout = unsafe {
            device.base.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder()
                    .flags(vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR)
                    .bindings(&[vk::DescriptorSetLayoutBinding::builder()
                        .binding(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                        .build()]),
                None,
            )
        }
        .expect("Failed to create descriptor set layout");

        let pipeline_layout = unsafe {
            device.base.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&[descriptor_layout])
                    .push_constant_ranges(&[vk::PushConstantRange::builder()
                        .size(64)
                        .stage_flags(vk::ShaderStageFlags::VERTEX)
                        .build()]),
                None,
            )
        }
        .expect("Failed to create pipeline layout");

        let pipeline = create_pipeline(&device.base, pipeline_layout);

        Self {
            imgui_context: Rc::new(RefCell::new(imgui_context)),
            winit_platform,
            device,
            texture_atlas,
            texture_sampler,
            descriptor_layout,
            pipeline_layout,
            pipeline,
            texture_atlas_data: Some(image_data),
        }
    }

    pub fn update_time(&mut self, last_frame: Instant) {
        self.imgui_context
            .borrow_mut()
            .io_mut()
            .update_delta_time(last_frame.elapsed());
    }

    pub fn handle_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::Event<()>,
    ) {
        self.winit_platform
            .handle_event(self.imgui_context.borrow_mut().io_mut(), window, event);
    }

    //TODO: callback for building ui
    pub fn build_frame(
        &mut self,
        window: &winit::window::Window,
        callback: impl FnOnce(&mut imgui::Ui),
    ) {
        let mut imgui_context = self.imgui_context.borrow_mut();

        self.winit_platform
            .prepare_frame(imgui_context.io_mut(), window)
            .expect("Failed to prepare frame");
        let ui = imgui_context.frame();

        callback(ui);

        self.winit_platform.prepare_render(ui, window);
    }

    pub fn end_frame_no_render(&mut self) {
        let mut imgui_context = self.imgui_context.borrow_mut();
        let _ = imgui_context.render();
    }

    pub fn build_render_pass(
        &mut self,
        rgb: &mut render_graph::RenderGraphBuilder,
        target_image: ImageHandle,
    ) {
        const MAX_QUAD_COUNT: usize = u16::MAX as usize;
        const MAX_VERTEX_COUNT: usize = MAX_QUAD_COUNT * 4;
        const MAX_INDEX_COUNT: usize = MAX_QUAD_COUNT * 6;
        let vertex_buffer = rgb.create_buffer(render_graph::BufferResourceDescription::New(
            BufferDescription {
                size: MAX_VERTEX_COUNT * 20, //TODO: size of vertex
                usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                memory_location: gpu_allocator::MemoryLocation::CpuToGpu,
            },
        ));

        let index_buffer = rgb.create_buffer(render_graph::BufferResourceDescription::New(
            BufferDescription {
                size: MAX_INDEX_COUNT * std::mem::size_of::<u16>(),
                usage: vk::BufferUsageFlags::INDEX_BUFFER,
                memory_location: gpu_allocator::MemoryLocation::CpuToGpu,
            },
        ));

        let pipeline_layout = self.pipeline_layout.clone();
        let pipeline = self.pipeline.clone();
        let imgui_context = self.imgui_context.clone();

        let texture_atlas = self.texture_atlas.clone_no_drop();
        let texture_atlas_sampler = self.texture_sampler.clone();
        let texture_atlas_data = self.texture_atlas_data.take();

        let mut imgui_pass = rgb.create_pass("ImguiPass");
        imgui_pass.buffer(vertex_buffer, render_graph::BufferAccessType::VertexBuffer);
        imgui_pass.buffer(index_buffer, render_graph::BufferAccessType::IndexBuffer);
        imgui_pass.raster(vec![(target_image, [0.0, 0.5, 1.0, 0.0])], None);
        imgui_pass.render(move |render_api, transfer_queue, pass_info, resources| {
            if let Some(image_data) = texture_atlas_data {
                transfer_queue.copy_to_image(
                    &texture_atlas,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    &image_data,
                );
            }

            //Get ref to imgui context
            let mut imgui_context = imgui_context.borrow_mut();

            let vertex_buffer = &resources.buffers[vertex_buffer as usize];
            let index_buffer = &resources.buffers[index_buffer as usize];

            //Prepare to draw frame
            let draw_data = imgui_context.render();

            // //Fill Buffers
            let (vertices, indices, offsets) = collect_mesh_buffers(&draw_data);
            vertex_buffer.fill(&vertices);
            index_buffer.fill(&indices);

            let i = 0;
            let framebuffer_size = pass_info.framebuffer_size.unwrap();

            let framebuffer_width = draw_data.framebuffer_scale[0] * draw_data.display_size[0];
            let framebuffer_height = draw_data.framebuffer_scale[1] * draw_data.display_size[1];

            unsafe {
                render_api.device.base.cmd_bind_pipeline(
                    render_api.command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline,
                );

                //TODO: other textures
                let image_info = vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(texture_atlas.view.unwrap())
                    .sampler(texture_atlas_sampler);
                let writes = &[vk::WriteDescriptorSet::builder()
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .dst_binding(0)
                    .dst_array_element(0)
                    .image_info(&[*image_info])
                    .build()];
                render_api.device.push_descriptor.cmd_push_descriptor_set(
                    render_api.command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline_layout,
                    0,
                    writes,
                );

                render_api.device.base.cmd_set_viewport(
                    render_api.command_buffer,
                    0,
                    &[vk::Viewport {
                        x: 0.0,
                        y: 0.0,
                        width: framebuffer_size.width as f32,
                        height: framebuffer_size.height as f32,
                        min_depth: 0.0,
                        max_depth: 1.0,
                    }],
                );

                //Push data
                let mut push_data = [0f32; 4];
                //Scale
                push_data[0] = 2.0 / draw_data.display_size[0];
                push_data[1] = 2.0 / draw_data.display_size[1];
                //Translate
                push_data[2] = -1.0 - (draw_data.display_pos[0] * push_data[0]);
                push_data[3] = -1.0 - (draw_data.display_pos[1] * push_data[1]);
                render_api.device.base.cmd_push_constants(
                    render_api.command_buffer,
                    pipeline_layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    any_as_u8_slice(&push_data),
                );

                //Bind buffers
                render_api.device.base.cmd_bind_vertex_buffers(
                    render_api.command_buffer,
                    0,
                    &[vertex_buffer.handle],
                    &[0],
                );
                render_api.device.base.cmd_bind_index_buffer(
                    render_api.command_buffer,
                    index_buffer.handle,
                    0,
                    vk::IndexType::UINT16,
                );

                let clip_offset = draw_data.display_pos;
                let clip_scale = draw_data.framebuffer_scale;

                let mut draw_index = 0;
                for draw_list in draw_data.draw_lists() {
                    for command in draw_list.commands() {
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
                                //TODO: texture_id
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

                                    let scissors = [vk::Rect2D {
                                        offset: vk::Offset2D {
                                            x: clip_rect[0] as i32,
                                            y: clip_rect[1] as i32,
                                        },
                                        extent: vk::Extent2D {
                                            width: (clip_rect[2] - clip_rect[0]) as u32,
                                            height: (clip_rect[3] - clip_rect[1]) as u32,
                                        },
                                    }];
                                    render_api.device.base.cmd_set_scissor(
                                        render_api.command_buffer,
                                        0,
                                        &scissors,
                                    );

                                    let (vertex_offset, index_offset) = offsets[draw_index];

                                    render_api.device.base.cmd_draw_indexed(
                                        render_api.command_buffer,
                                        count as _,
                                        1,
                                        index_offset + idx_offset as u32,
                                        vertex_offset + vtx_offset as i32,
                                        0,
                                    )
                                }
                            }
                            DrawCmd::ResetRenderState => {}
                            DrawCmd::RawCallback { .. } => {}
                        }
                    }
                    draw_index += 1;
                }
            }
        });
    }
}

impl Drop for ImguiLayer {
    fn drop(&mut self) {
        unsafe {
            self.device.base.destroy_pipeline(self.pipeline, None);
            self.device
                .base
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device
                .base
                .destroy_descriptor_set_layout(self.descriptor_layout, None);
        }
    }
}

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

fn collect_mesh_buffers(draw_data: &DrawData) -> (Vec<DrawVert>, Vec<DrawIdx>, Vec<(i32, u32)>) {
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

fn read_shader_from_source(source: &[u8]) -> Vec<u32> {
    use std::io::Cursor;
    let mut cursor = Cursor::new(source);
    ash::util::read_spv(&mut cursor).expect("Failed to read spv")
}

fn create_pipeline(device: &Rc<ash::Device>, pipeline_layout: vk::PipelineLayout) -> vk::Pipeline {
    let entry_point_name = CString::new("main").unwrap();

    let vertex_shader_code =
        read_shader_from_source(std::include_bytes!("../shaders/imgui.vert.spv"));
    let fragment_shader_code =
        read_shader_from_source(std::include_bytes!("../shaders/imgui.frag.spv"));

    let vertex_module = unsafe {
        device.create_shader_module(
            &vk::ShaderModuleCreateInfo::builder().code(&vertex_shader_code),
            None,
        )
    }
    .expect("Failed to create vertex module");

    let fragment_module = unsafe {
        device.create_shader_module(
            &vk::ShaderModuleCreateInfo::builder().code(&fragment_shader_code),
            None,
        )
    }
    .expect("Failed to create fragment module");

    let shader_states_infos = [
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_module)
            .name(&entry_point_name)
            .build(),
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_module)
            .name(&entry_point_name)
            .build(),
    ];

    let binding_desc = [vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(20)
        .input_rate(vk::VertexInputRate::VERTEX)
        .build()];
    let attribute_desc = [
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0)
            .build(),
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(8)
            .build(),
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R8G8B8A8_UNORM)
            .offset(16)
            .build(),
    ];

    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(&binding_desc)
        .vertex_attribute_descriptions(&attribute_desc);

    let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::CLOCKWISE)
        .depth_bias_enable(false)
        .depth_bias_constant_factor(0.0)
        .depth_bias_clamp(0.0)
        .depth_bias_slope_factor(0.0);

    let viewports = [Default::default()];
    let scissors = [Default::default()];
    let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(&viewports)
        .scissors(&scissors);

    let multisampling_info = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1)
        .min_sample_shading(1.0)
        .alpha_to_coverage_enable(false)
        .alpha_to_one_enable(false);

    let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(
            vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
        )
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD)
        .build()];
    let color_blending_info = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .logic_op(vk::LogicOp::COPY)
        .attachments(&color_blend_attachments)
        .blend_constants([0.0, 0.0, 0.0, 0.0]);

    let dynamic_states = [vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT];
    let dynamic_states_info =
        vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

    let color_formats = &[vk::Format::B8G8R8A8_UNORM];
    let mut dynamic_rendering_info = vk::PipelineRenderingCreateInfoKHR::builder()
        .view_mask(1)
        .color_attachment_formats(color_formats)
        .build();

    let pipeline_info = &[vk::GraphicsPipelineCreateInfo::builder()
        .stages(&shader_states_infos)
        .vertex_input_state(&vertex_input_info)
        .input_assembly_state(&input_assembly_info)
        .rasterization_state(&rasterizer_info)
        .viewport_state(&viewport_info)
        .multisample_state(&multisampling_info)
        .color_blend_state(&color_blending_info)
        .dynamic_state(&dynamic_states_info)
        .layout(pipeline_layout)
        //.render_pass(render_pass)
        .render_pass(vk::RenderPass::null())
        .subpass(0)
        .push_next(&mut dynamic_rendering_info)
        .build()];

    let pipeline =
        unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), pipeline_info, None) }
            .expect("Failed to create pipeline")[0];

    unsafe {
        device.destroy_shader_module(vertex_module, None);
        device.destroy_shader_module(fragment_module, None);
    }

    pipeline
}
