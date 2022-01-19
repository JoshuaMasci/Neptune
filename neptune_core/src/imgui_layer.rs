use imgui_winit_support::{HiDpiMode, WinitPlatform};
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

struct Frame {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

pub struct ImguiLayer {
    imgui_context: imgui::Context,
    winit_platform: WinitPlatform,

    device: RenderDevice,

    texture_atlas_staging_buffer: Option<Buffer>,
    old_staging_buffer: Option<Buffer>,

    texture_atlas: Image,
    texture_sampler: vk::Sampler,
    pub framebuffer_set: FrameBufferSet,
    descriptor_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,

    frames: Vec<Frame>,
}

impl ImguiLayer {
    pub fn new(window: &winit::window::Window, device: RenderDevice) -> Self {
        let mut imgui_context = imgui::Context::create();
        let mut winit_platform = WinitPlatform::init(&mut imgui_context);
        winit_platform.attach_window(imgui_context.io_mut(), window, HiDpiMode::Default);

        //Config imgui
        imgui_context.io_mut().config_flags |= imgui::ConfigFlags::DOCKING_ENABLE;
        imgui_context.set_ini_filename(None);
        imgui_context.set_renderer_name(Some(String::from("Neptune Renderer")));

        let image_data = imgui_context.fonts().build_alpha8_texture();
        let texture_atlas_staging_buffer = Some({
            let buffer = Buffer::new(
                &device,
                BufferDescription {
                    size: image_data.data.len(),
                    usage: vk::BufferUsageFlags::TRANSFER_SRC,
                    memory_location: MemoryLocation::CpuToGpu,
                },
            );
            buffer.fill(image_data.data);
            buffer
        });

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

        let size = window.inner_size();
        let framebuffer_set = FrameBufferSet::new(
            &device,
            vk::Extent2D {
                width: size.width,
                height: size.height,
            },
            vec![vk::Format::R8G8B8A8_UNORM],
            None,
            1,
        );

        let pipeline = create_pipeline(&device.base, pipeline_layout, framebuffer_set.render_pass);

        //Will be resized during first frame
        let frames = vec![Frame {
            vertex_buffer: Buffer::new(
                &device,
                BufferDescription {
                    size: 16,
                    usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                    memory_location: MemoryLocation::CpuToGpu,
                },
            ),
            index_buffer: Buffer::new(
                &device,
                BufferDescription {
                    size: 16,
                    usage: vk::BufferUsageFlags::INDEX_BUFFER,
                    memory_location: MemoryLocation::CpuToGpu,
                },
            ),
        }];

        Self {
            imgui_context,
            winit_platform,
            device,
            texture_atlas_staging_buffer,
            old_staging_buffer: None,
            texture_atlas,
            texture_sampler,
            framebuffer_set,
            descriptor_layout,
            pipeline_layout,
            pipeline,
            frames,
        }
    }

    pub fn update_time(&mut self, last_frame: Instant) {
        self.imgui_context
            .io_mut()
            .update_delta_time(last_frame.elapsed());
    }

    pub fn handle_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::Event<()>,
    ) {
        self.winit_platform
            .handle_event(self.imgui_context.io_mut(), window, event);
    }

    pub fn build_render_pass(
        &mut self,
        window: &winit::window::Window,
        rgb: &mut render_graph::RenderGraphBuilder,
    ) -> ImageHandle {
        0
    }

    pub fn render_frame(
        &mut self,
        window: &winit::window::Window,
        command_buffer: vk::CommandBuffer,
    ) {
        //TODO: Transfer image data better
        let _ = self.old_staging_buffer.take();
        if let Some(staging_buffer) = self.texture_atlas_staging_buffer.take() {
            let image_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_array_layer(0)
                .layer_count(1)
                .base_mip_level(0)
                .level_count(1)
                .build();

            let image_barriers1 = &[vk::ImageMemoryBarrier2KHR::builder()
                .image(self.texture_atlas.handle)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .src_access_mask(vk::AccessFlags2KHR::NONE)
                .src_stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_access_mask(vk::AccessFlags2KHR::NONE)
                .dst_stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .subresource_range(image_range)
                .build()];

            unsafe {
                self.device.synchronization2.cmd_pipeline_barrier2(
                    command_buffer,
                    &vk::DependencyInfoKHR::builder().image_memory_barriers(image_barriers1),
                );
            }

            unsafe {
                self.device.base.cmd_copy_buffer_to_image(
                    command_buffer,
                    staging_buffer.handle,
                    self.texture_atlas.handle,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[vk::BufferImageCopy {
                        buffer_offset: 0,
                        buffer_row_length: 0,
                        buffer_image_height: 0,
                        image_subresource: vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            mip_level: 0,
                            base_array_layer: 0,
                            layer_count: 1,
                        },
                        image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
                        image_extent: vk::Extent3D {
                            width: self.texture_atlas.description.size[0],
                            height: self.texture_atlas.description.size[1],
                            depth: 1,
                        },
                    }],
                );
            }

            let image_barriers2 = &[vk::ImageMemoryBarrier2KHR::builder()
                .image(self.texture_atlas.handle)
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .src_access_mask(vk::AccessFlags2KHR::NONE)
                .src_stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_access_mask(vk::AccessFlags2KHR::NONE)
                .dst_stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .subresource_range(image_range)
                .build()];

            unsafe {
                self.device.synchronization2.cmd_pipeline_barrier2(
                    command_buffer,
                    &vk::DependencyInfoKHR::builder().image_memory_barriers(image_barriers2),
                );
            }
            self.old_staging_buffer = Some(staging_buffer);
        }

        self.winit_platform
            .prepare_frame(self.imgui_context.io_mut(), window)
            .expect("Failed to prepare frame");
        let ui = self.imgui_context.frame();

        //TODO: enable docking
        crate::imgui_docking::enable_docking();

        if let Some(menu_bar) = ui.begin_main_menu_bar() {
            if let Some(menu) = ui.begin_menu("Options") {
                menu.end();
            }

            menu_bar.end();
        }

        ui.window("Example Window")
            .size([100.0, 50.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("An example");
            });

        let mut run = true;
        ui.show_demo_window(&mut run);

        self.winit_platform.prepare_render(ui, window);

        let draw_data = self.imgui_context.render();

        let (vertices, indices, offsets) = collect_mesh_buffers(&draw_data);

        let vertex_size = vertices.len() * std::mem::size_of::<imgui::DrawVert>();
        let index_size = indices.len() * std::mem::size_of::<u16>();

        let frame = &mut self.frames[0];

        //Resize buffers
        if frame.vertex_buffer.description.size < vertex_size {
            frame.vertex_buffer = Buffer::new(
                &self.device,
                BufferDescription {
                    size: vertex_size,
                    usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                    memory_location: MemoryLocation::CpuToGpu,
                },
            );
        }

        if frame.index_buffer.description.size < index_size {
            frame.index_buffer = Buffer::new(
                &self.device,
                BufferDescription {
                    size: index_size,
                    usage: vk::BufferUsageFlags::INDEX_BUFFER,
                    memory_location: MemoryLocation::CpuToGpu,
                },
            );
        }

        //Fill Buffers
        frame.vertex_buffer.fill(&vertices);
        frame.index_buffer.fill(&indices);

        let framebuffer_width = draw_data.framebuffer_scale[0] * draw_data.display_size[0];
        let framebuffer_height = draw_data.framebuffer_scale[1] * draw_data.display_size[1];

        self.framebuffer_set.set_size(vk::Extent2D {
            width: framebuffer_width as u32,
            height: framebuffer_height as u32,
        });
        self.framebuffer_set.update_frame(0);

        //Draw UI
        unsafe {
            let clear_values = &[vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }];

            self.device.base.cmd_begin_render_pass(
                command_buffer,
                &vk::RenderPassBeginInfo::builder()
                    .render_pass(self.framebuffer_set.render_pass)
                    .framebuffer(self.framebuffer_set.framebuffers[0].handle)
                    .render_area(vk::Rect2D {
                        offset: Offset2D { x: 0, y: 0 },
                        extent: self.framebuffer_set.current_size,
                    })
                    .clear_values(clear_values),
                vk::SubpassContents::INLINE,
            );

            self.device.base.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            self.device.base.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: framebuffer_width,
                    height: framebuffer_height,
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
            self.device.base.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                any_as_u8_slice(&push_data),
            );

            //Bind buffers
            self.device.base.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[frame.vertex_buffer.handle],
                &[0],
            );
            self.device.base.cmd_bind_index_buffer(
                command_buffer,
                frame.index_buffer.handle,
                0,
                vk::IndexType::UINT16,
            );

            //TODO: other textures
            let image_info = vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(self.texture_atlas.view)
                .sampler(self.texture_sampler);
            let writes = &[vk::WriteDescriptorSet::builder()
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .dst_binding(0)
                .dst_array_element(0)
                .image_info(&[*image_info])
                .build()];
            self.device.push_descriptor.cmd_push_descriptor_set(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                writes,
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
                                self.device
                                    .base
                                    .cmd_set_scissor(command_buffer, 0, &scissors);

                                let (vertex_offset, index_offset) = offsets[draw_index];

                                self.device.base.cmd_draw_indexed(
                                    command_buffer,
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

            self.device.base.cmd_end_render_pass(command_buffer);
        }
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

fn create_pipeline(
    device: &Rc<ash::Device>,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
) -> vk::Pipeline {
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
        .render_pass(render_pass)
        .subpass(0)
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
