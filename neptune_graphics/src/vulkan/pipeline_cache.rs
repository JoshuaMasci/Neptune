use crate::pipeline::{BlendFactor, BlendOp, CullMode, DepthTestOp, PipelineState, VertexElement};
use crate::vulkan::ShaderModule;
use ash::vk;
use std::collections::HashMap;
use std::ffi::CString;
use std::rc::Rc;

//TODO: Allow more complicated vertex layouts
#[derive(Hash, Eq, PartialEq)]
pub struct VertexLayout {
    vertex_elements: Vec<VertexElement>,
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct FramebufferLayout {
    pub color_attachments: Vec<vk::Format>,
    pub depth_stencil_attachment: Option<vk::Format>,
}

#[derive(Hash, Eq, PartialEq)]
struct GraphicsPipelineHash {
    vertex_module: vk::ShaderModule,
    fragment_module: Option<vk::ShaderModule>,
    vertex_elements: Vec<VertexElement>,
    state: crate::pipeline::PipelineState,
    framebuffer_layout: FramebufferLayout,
}

pub struct PipelineCache {
    device: Rc<ash::Device>,
    pub(crate) pipeline_layout: vk::PipelineLayout,
    graphics_pipelines: HashMap<GraphicsPipelineHash, vk::Pipeline>,
}

impl PipelineCache {
    pub fn new(device: Rc<ash::Device>, pipeline_layout: vk::PipelineLayout) -> Self {
        Self {
            device,
            pipeline_layout,
            graphics_pipelines: HashMap::new(),
        }
    }

    pub fn get_graphics(
        &mut self,
        vertex_module: Rc<ShaderModule>,
        fragment_module: Option<Rc<ShaderModule>>,
        vertex_elements: Vec<VertexElement>,
        state: crate::pipeline::PipelineState,
        framebuffer_layout: FramebufferLayout,
    ) -> vk::Pipeline {
        let pipeline_hash = GraphicsPipelineHash {
            vertex_module: vertex_module.module,
            fragment_module: fragment_module.map(|module| module.module),
            vertex_elements,
            state,
            framebuffer_layout,
        };

        if let Some(&pipeline) = self.graphics_pipelines.get(&pipeline_hash) {
            pipeline
        } else {
            //TODO: build pipeline
            let new_pipeline = self.create_graphics_pipeline(&pipeline_hash);
            let _ = self.graphics_pipelines.insert(pipeline_hash, new_pipeline);
            new_pipeline
        }
    }

    fn create_graphics_pipeline(
        &mut self,
        pipeline_description: &GraphicsPipelineHash,
    ) -> vk::Pipeline {
        let entry_point_name = CString::new("main").unwrap();

        let mut shader_states_infos: Vec<vk::PipelineShaderStageCreateInfo> =
            vec![vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(pipeline_description.vertex_module)
                .name(&entry_point_name)
                .build()];

        if let Some(fragment_module) = pipeline_description.fragment_module {
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

        let mut vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder();
        if !pipeline_description.vertex_elements.is_empty() {
            vertex_input_info = vertex_input_info
                .vertex_binding_descriptions(&binding_desc)
                .vertex_attribute_descriptions(&attribute_description);
        }

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(pipeline_description.state.cull_mode.get_vk_type())
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
            .blend_enable(pipeline_description.state.blend_op != BlendOp::None)
            .src_color_blend_factor(pipeline_description.state.src_factor.get_vk_type())
            .dst_color_blend_factor(pipeline_description.state.dst_factor.get_vk_type())
            .color_blend_op(pipeline_description.state.blend_op.get_vk_type())
            .src_alpha_blend_factor(pipeline_description.state.src_factor.get_vk_type())
            .dst_alpha_blend_factor(pipeline_description.state.dst_factor.get_vk_type())
            .alpha_blend_op(pipeline_description.state.blend_op.get_vk_type())
            .build()];
        let color_blending_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&color_blend_attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let dynamic_states = [vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT];
        let dynamic_states_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

        let mut dynamic_rendering_info = vk::PipelineRenderingCreateInfoKHR::builder()
            .view_mask(0)
            .color_attachment_formats(&pipeline_description.framebuffer_layout.color_attachments);

        if let Some(depth_format) = pipeline_description
            .framebuffer_layout
            .depth_stencil_attachment
        {
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
            self.device
                .create_graphics_pipelines(vk::PipelineCache::null(), pipeline_info, None)
        }
        .expect("Failed to create pipeline")[0]
    }
}

impl Drop for PipelineCache {
    fn drop(&mut self) {
        for (_, pipeline) in self.graphics_pipelines.drain() {
            unsafe {
                self.device.destroy_pipeline(pipeline, None);
            }
        }
    }
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

impl DepthTestOp {
    fn get_vk_type(&self) {
        todo!();
    }
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

impl VertexElement {
    fn get_vk_type(&self) -> vk::Format {
        match self {
            VertexElement::Byte => vk::Format::R8_UNORM,
            VertexElement::Byte2 => vk::Format::R8G8_UNORM,
            VertexElement::Byte3 => vk::Format::R8G8B8_UNORM,
            VertexElement::Byte4 => vk::Format::R8G8B8A8_UNORM,
            VertexElement::Float => vk::Format::R32_SFLOAT,
            VertexElement::Float2 => vk::Format::R32G32_SFLOAT,
            VertexElement::Float3 => vk::Format::R32G32B32_SFLOAT,
            VertexElement::Float4 => vk::Format::R32G32B32A32_SFLOAT,
        }
    }
}
