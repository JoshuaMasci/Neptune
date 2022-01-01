use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::cell::RefCell;
use std::ffi::CString;
use std::rc::Rc;
use std::time::Instant;

use crate::buffer::Buffer;
use crate::framebuffer::FrameBufferSet;
use crate::image::Image;
use ash::vk;
use ash::vk::Offset2D;
use gpu_allocator::MemoryLocation;

pub struct ImguiLayer {
    imgui_context: imgui::Context,
    winit_platform: WinitPlatform,

    device: ash::Device,
    device_allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,

    texture_atlas_staging_buffer: Option<Buffer>,
    texture_atlas: Image,

    pub framebuffer_set: FrameBufferSet,

    descriptor_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
}

impl ImguiLayer {
    pub fn new(
        window: &winit::window::Window,
        device: ash::Device,
        device_allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,
    ) -> Self {
        let mut imgui_context = imgui::Context::create();
        let mut winit_platform = WinitPlatform::init(&mut imgui_context);
        winit_platform.attach_window(imgui_context.io_mut(), window, HiDpiMode::Default);

        //TODO: load image
        let image_data = imgui_context.fonts().build_alpha8_texture();

        let texture_atlas_staging_buffer = Some({
            let mut buffer = Buffer::new(
                device.clone(),
                device_allocator.clone(),
                &vk::BufferCreateInfo::builder()
                    .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                    .size(image_data.data.len() as vk::DeviceSize),
                MemoryLocation::CpuToGpu,
            );
            buffer.fill(image_data.data);
            buffer
        });

        let texture_atlas = Image::new_2d(
            device.clone(),
            device_allocator.clone(),
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            vk::Format::R8_UINT,
            vk::Extent2D {
                width: image_data.width,
                height: image_data.height,
            },
            MemoryLocation::GpuOnly,
        );

        let descriptor_layout = unsafe {
            device.create_descriptor_set_layout(
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
            device.create_pipeline_layout(
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
            device.clone(),
            device_allocator.clone(),
            vk::Extent2D {
                width: size.width,
                height: size.height,
            },
            vec![vk::Format::R8G8B8A8_UNORM],
            None,
            1,
        );

        let pipeline = create_pipeline(&device, pipeline_layout, framebuffer_set.render_pass);

        Self {
            imgui_context,
            winit_platform,
            device,
            device_allocator,
            texture_atlas_staging_buffer,
            texture_atlas,
            framebuffer_set,
            descriptor_layout,
            pipeline_layout,
            pipeline,
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

    pub fn render_frame(
        &mut self,
        window: &winit::window::Window,
        command_buffer: vk::CommandBuffer,
    ) {
        self.winit_platform
            .prepare_frame(self.imgui_context.io_mut(), window)
            .expect("Failed to prepare frame");
        let frame = self.imgui_context.frame();

        let mut run = true;
        frame.show_demo_window(&mut run);

        self.winit_platform.prepare_render(frame, window);
        let _draw_data = self.imgui_context.render();
        //TODO: draw frame here

        unsafe {
            let clear_values = &[vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [1.0, 0.0, 1.0, 1.0],
                },
            }];

            self.device.cmd_begin_render_pass(
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

            self.device.cmd_end_render_pass(command_buffer);
        }
    }
}

impl Drop for ImguiLayer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_layout, None);
        }
    }
}

fn read_shader_from_source(source: &[u8]) -> Vec<u32> {
    use std::io::Cursor;
    let mut cursor = Cursor::new(source);
    ash::util::read_spv(&mut cursor).expect("Failed to read spv")
}

fn create_pipeline(
    device: &ash::Device,
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
        .color_write_mask(vk::ColorComponentFlags::all())
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
