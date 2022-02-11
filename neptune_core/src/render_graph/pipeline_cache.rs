use crate::render_backend::RenderDevice;
use ash::vk;
use std::collections::HashMap;
use std::ffi::CString;

#[derive(Hash, Eq, PartialEq)]
pub struct FramebufferLayout {
    color_attachments: Vec<vk::Format>,
    depth_attachment: Option<vk::Format>,
}

pub struct PipelineCache {
    device: RenderDevice,
    pipeline_layout: vk::PipelineLayout,

    graphics_pipelines: HashMap<GraphicsPipelineDescription, vk::Pipeline>,
}

impl PipelineCache {
    pub fn get_graphics(
        &mut self,
        pipeline_description: &GraphicsPipelineDescription,
        layout: &FramebufferLayout,
    ) -> vk::Pipeline {
        if let Some(&pipeline) = self.graphics_pipelines.get(pipeline_description) {
            pipeline
        } else {
            //TODO: build pipeline
            let new_pipeline = self.create_graphics_pipeline(pipeline_description, layout);
            // let _ = self
            //     .graphics_pipelines
            //     .insert((*pipeline_description).clone(), pipeline);
            new_pipeline
        }
    }

    fn create_graphics_pipeline(
        &mut self,
        pipeline_description: &GraphicsPipelineDescription,
        layout: &FramebufferLayout,
    ) -> vk::Pipeline {
        let entry_point_name = CString::new("main").unwrap();

        let mut shader_states_infos: Vec<vk::PipelineShaderStageCreateInfo> =
            vec![vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(pipeline_description.vertex_module)
                .name(&entry_point_name)
                .build()];

        if let Some(fragment_module) = pipeline_description.fragment_shader {
            shader_states_infos.push(
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(fragment_module)
                    .name(&entry_point_name)
                    .build(),
            );
        }

        let mut total_size = 0u32;
        let attribute_description: Vec<vk::VertexInputAttributeDescription> = pipeline_description
            .vertex_elements
            .iter()
            .enumerate()
            .map(|(index, element)| {
                let attribute_description = vk::VertexInputAttributeDescription::builder()
                    .binding(0)
                    .location(index as u32)
                    .format(element.get_vk_type())
                    .offset(total_size)
                    .build();
                total_size += element.get_size_bytes();
                attribute_description
            })
            .collect();

        let binding_desc = [vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(total_size)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&binding_desc)
            .vertex_attribute_descriptions(&attribute_description);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(pipeline_description.cull_mode.get_vk_type())
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
            .blend_enable(pipeline_description.blend_op != BlendOp::None)
            .src_color_blend_factor(pipeline_description.src_factor.get_vk_type())
            .dst_color_blend_factor(pipeline_description.dst_factor.get_vk_type())
            .color_blend_op(pipeline_description.blend_op.get_vk_type())
            .src_alpha_blend_factor(pipeline_description.src_factor.get_vk_type())
            .dst_alpha_blend_factor(pipeline_description.dst_factor.get_vk_type())
            .alpha_blend_op(pipeline_description.blend_op.get_vk_type())
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
            .color_attachment_formats(&layout.color_attachments);

        if let Some(depth_format) = layout.depth_attachment {
            dynamic_rendering_info = dynamic_rendering_info.depth_attachment_format(depth_format);
        }

        let pipeline_info = &[vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_states_infos)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .rasterization_state(&rasterizer_info)
            .viewport_state(&viewport_info)
            .multisample_state(&multisampling_info)
            .color_blend_state(&color_blending_info)
            .dynamic_state(&dynamic_states_info)
            .layout(self.pipeline_layout)
            .render_pass(vk::RenderPass::null())
            .subpass(0)
            .push_next(&mut dynamic_rendering_info)
            .build()];

        unsafe {
            self.device.base.create_graphics_pipelines(
                vk::PipelineCache::null(),
                pipeline_info,
                None,
            )
        }
        .expect("Failed to create pipeline")[0]
    }
}

#[derive(Hash, Eq, PartialEq)]
pub enum CullMode {
    None,
    Front,
    Back,
    All,
}

impl CullMode {
    fn get_vk_type(&self) -> vk::CullModeFlags {
        match self {
            CullMode::None => vk::CullModeFlags::NONE,
            CullMode::Front => vk::CullModeFlags::FRONT,
            CullMode::Back => vk::CullModeFlags::BACK,
            CullMode::All => vk::CullModeFlags::FRONT_AND_BACK,
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
pub enum DepthTestMode {
    None,
    TestOnly,
    TestAndWrite,
}

#[derive(Hash, Eq, PartialEq)]
pub enum DepthTestOp {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

impl DepthTestOp {
    fn get_vk_type(&self) {
        todo!();
    }
}

#[derive(Hash, Eq, PartialEq)]
pub enum BlendFactor {
    Zero,
    One,
    ColorSrc,
    OneMinusColorSrc,
    ColorDst,
    OneMinusColorDst,
    AlphaSrc,
    OneMinusAlphaSrc,
    AlphaDst,
    OneMinusAlphaDst,
}

impl BlendFactor {
    fn get_vk_type(&self) -> vk::BlendFactor {
        match self {
            BlendFactor::Zero => vk::BlendFactor::ZERO,
            BlendFactor::One => vk::BlendFactor::ONE,
            BlendFactor::ColorSrc => vk::BlendFactor::SRC_COLOR,
            BlendFactor::OneMinusColorSrc => vk::BlendFactor::ONE_MINUS_SRC_COLOR,
            BlendFactor::ColorDst => vk::BlendFactor::DST_COLOR,
            BlendFactor::OneMinusColorDst => vk::BlendFactor::ONE_MINUS_DST_COLOR,
            BlendFactor::AlphaSrc => vk::BlendFactor::SRC_ALPHA,
            BlendFactor::OneMinusAlphaSrc => vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            BlendFactor::AlphaDst => vk::BlendFactor::DST_ALPHA,
            BlendFactor::OneMinusAlphaDst => vk::BlendFactor::ONE_MINUS_DST_ALPHA,
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
pub enum BlendOp {
    None,
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

impl BlendOp {
    fn get_vk_type(&self) -> vk::BlendOp {
        match self {
            BlendOp::None => vk::BlendOp::ADD, //Blending will be disabled for this
            BlendOp::Add => vk::BlendOp::ADD,
            BlendOp::Subtract => vk::BlendOp::SUBTRACT,
            BlendOp::ReverseSubtract => vk::BlendOp::REVERSE_SUBTRACT,
            BlendOp::Min => vk::BlendOp::MIN,
            BlendOp::Max => vk::BlendOp::MAX,
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
pub enum VertexElement {
    float,
    float2,
    float3,
    float4,
}

impl VertexElement {
    fn get_size_bytes(&self) -> u32 {
        let float_size = std::mem::size_of::<f32>() as u32;
        match self {
            VertexElement::float => float_size,
            VertexElement::float2 => float_size * 2,
            VertexElement::float3 => float_size * 3,
            VertexElement::float4 => float_size * 4,
        }
    }

    fn get_vk_type(&self) -> vk::Format {
        match self {
            VertexElement::float => vk::Format::R32_SFLOAT,
            VertexElement::float2 => vk::Format::R32G32_SFLOAT,
            VertexElement::float3 => vk::Format::R32G32B32_SFLOAT,
            VertexElement::float4 => vk::Format::R32G32B32A32_SFLOAT,
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
pub struct GraphicsPipelineDescription {
    vertex_module: vk::ShaderModule,
    fragment_shader: Option<vk::ShaderModule>,
    cull_mode: CullMode,
    depth_mode: DepthTestMode,
    depth_op: DepthTestOp,
    src_factor: BlendFactor,
    dst_factor: BlendFactor,
    blend_op: BlendOp, //TODO: add alpha channel blending
    vertex_elements: Vec<VertexElement>,
}
