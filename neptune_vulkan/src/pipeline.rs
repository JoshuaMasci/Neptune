use crate::VulkanError;
use ash::vk;

//TODO: put blending config in here as well
pub struct FramebufferDesc<'a> {
    pub color_attachments: &'a [vk::Format],
    pub depth_attachment: Option<vk::Format>,
    pub stencil_attachment: Option<vk::Format>,
}

pub struct VertexInputDesc {
    pub format: vk::Format,
    pub stride: u32,
}

pub struct RasterPipelineDesc<'a> {
    pub framebuffer: FramebufferDesc<'a>,
    pub vertex_input: VertexInputDesc,
    pub vertex_shader: &'a [u32],
    pub fragment_shader: Option<&'a [u32]>,
}

pub(crate) fn create_pipeline(
    device: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
    desc: &RasterPipelineDesc,
) -> Result<vk::Pipeline, VulkanError> {
    let vertex_shader_module = unsafe {
        device.create_shader_module(
            &vk::ShaderModuleCreateInfo::builder().code(desc.vertex_shader),
            None,
        )
    }?;
    let fragment_shader_module = if let Some(fragment_shader) = desc.fragment_shader {
        Some(unsafe {
            device.create_shader_module(
                &vk::ShaderModuleCreateInfo::builder().code(fragment_shader),
                None,
            )
        }?)
    } else {
        None
    };

    let entry_point_name = std::ffi::CString::new("main").unwrap();

    let mut shader_stages = vec![vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vertex_shader_module)
        .name(&entry_point_name)
        .build()];

    if let Some(fragment_shader_module) = fragment_shader_module {
        shader_stages.push(
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment_shader_module)
                .name(&entry_point_name)
                .build(),
        );
    }

    //TODO: allow config
    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false)
        .build();

    let vertex_binding_desc = [vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(desc.vertex_input.stride)
        .input_rate(vk::VertexInputRate::VERTEX)
        .build()];
    let vertex_attribute_desc = [vk::VertexInputAttributeDescription::builder()
        .binding(0)
        .location(0)
        .format(desc.vertex_input.format)
        .offset(0)
        .build()];
    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(&vertex_binding_desc)
        .vertex_attribute_descriptions(&vertex_attribute_desc);

    //Since dynamic states will be used here, the following values are just placeholders
    let viewports = [vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
        min_depth: 0.0,
        max_depth: 1.0,
    }];
    let scissors = [vk::Rect2D {
        offset: vk::Offset2D::default(),
        extent: vk::Extent2D {
            width: 1,
            height: 1,
        },
    }];
    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(&viewports)
        .scissors(&scissors);

    //TODO: allow config
    let rasterizer_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false)
        .depth_bias_constant_factor(0.0)
        .depth_bias_clamp(0.0)
        .depth_bias_slope_factor(0.0);

    //Msaa is probably not going to be supported at all. Most modern engines use other AA methods anyways
    let multisampling_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1)
        .min_sample_shading(1.0)
        .alpha_to_coverage_enable(false)
        .alpha_to_one_enable(false);

    //TODO: allow config
    let mut depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder();
    if desc.framebuffer.depth_attachment.is_some() {
        depth_stencil_state = depth_stencil_state
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0)
    }
    if desc.framebuffer.stencil_attachment.is_some() {
        //TODO: configure stencil
        depth_stencil_state = depth_stencil_state
            .stencil_test_enable(false)
            .front(vk::StencilOpState::default())
            .back(vk::StencilOpState::default());
    }

    //TODO: allow config
    let color_blend_attachments: Vec<vk::PipelineColorBlendAttachmentState> = desc
        .framebuffer
        .color_attachments
        .iter()
        .map(|_format| {
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .blend_enable(false)
                .build()
        })
        .collect();
    let color_blending_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .attachments(&color_blend_attachments)
        .build();

    //TODO: allow depth bias (and other?) dynamic state
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
        .dynamic_states(&dynamic_states)
        .build();

    let mut dynamic_rendering = vk::PipelineRenderingCreateInfo::builder()
        .color_attachment_formats(&desc.framebuffer.color_attachments);
    if let Some(depth_format) = desc.framebuffer.depth_attachment {
        dynamic_rendering = dynamic_rendering.depth_attachment_format(depth_format);
    }
    if let Some(stencil_format) = desc.framebuffer.stencil_attachment {
        dynamic_rendering = dynamic_rendering.stencil_attachment_format(stencil_format);
    }

    let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
        .push_next(&mut dynamic_rendering)
        .stages(&shader_stages)
        .input_assembly_state(&input_assembly_state)
        .vertex_input_state(&vertex_input_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterizer_state)
        .multisample_state(&multisampling_state)
        .depth_stencil_state(&depth_stencil_state)
        .color_blend_state(&color_blending_state)
        .dynamic_state(&dynamic_state)
        .layout(pipeline_layout);

    let pipeline = unsafe {
        device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            &[pipeline_create_info.build()],
            None,
        )
    }
    .unwrap()[0];

    unsafe {
        device.destroy_shader_module(vertex_shader_module, None);
        if let Some(fragment_shader_module) = fragment_shader_module {
            device.destroy_shader_module(fragment_shader_module, None);
        }
    }

    Ok(pipeline)
}
