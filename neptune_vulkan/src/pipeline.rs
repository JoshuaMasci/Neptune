use crate::device::AshDevice;
use crate::{ComputePipelineKey, RasterPipleineKey, VulkanError};
use ash::vk;
use slotmap::SlotMap;
use std::sync::Arc;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct ShaderStage<'a> {
    pub code: &'a [u32], //TODO: Shader Module
    pub entry: &'a str,
}

pub struct ComputePipeline {
    device: Arc<AshDevice>,
    pub handle: vk::Pipeline,
}

impl ComputePipeline {
    pub fn new(
        device: Arc<AshDevice>,
        pipeline_layout: vk::PipelineLayout,
        shader: &ShaderStage,
    ) -> Result<Self, VulkanError> {
        let compute_shader_module = unsafe {
            device.core.create_shader_module(
                &vk::ShaderModuleCreateInfo::builder().code(shader.code),
                None,
            )
        }?;

        let compute_entry_point_name = std::ffi::CString::new(shader.entry).unwrap();

        let compute_shader_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(compute_shader_module)
            .name(&compute_entry_point_name);

        let result = match unsafe {
            device.core.create_compute_pipelines(
                vk::PipelineCache::null(),
                &[vk::ComputePipelineCreateInfo::builder()
                    .stage(*compute_shader_stage)
                    .layout(pipeline_layout)
                    .build()],
                None,
            )
        } {
            Ok(pipelines) => Ok(Self {
                device: device.clone(),
                handle: pipelines[0],
            }),
            Err((_, err)) => return Err(err.into()),
        };

        unsafe {
            device
                .core
                .destroy_shader_module(compute_shader_module, None);
        }

        result
    }
}

impl Drop for ComputePipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.core.destroy_pipeline(self.handle, None);
        }
    }
}

//TODO: put blending config in here as well
pub struct FramebufferDesc<'a> {
    pub color_attachments: &'a [vk::Format],
    pub depth_attachment: Option<vk::Format>,
    pub stencil_attachment: Option<vk::Format>,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct VertexAttribute {
    pub shader_location: u32,
    pub format: vk::Format,
    pub offset: u32,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct VertexBufferLayout<'a> {
    pub stride: u32,
    pub input_rate: vk::VertexInputRate,
    pub attributes: &'a [VertexAttribute],
}

//TODO: make enum for mesh shading?
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct VertexState<'a> {
    pub shader: ShaderStage<'a>,
    pub layouts: &'a [VertexBufferLayout<'a>],
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct PrimitiveState {
    pub front_face: vk::FrontFace,
    pub cull_mode: vk::CullModeFlags,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct DepthState {
    pub format: vk::Format,
    pub depth_enabled: bool,
    pub write_depth: bool,
    pub depth_op: vk::CompareOp,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct ColorTargetState {
    pub format: vk::Format,
    pub blend: Option<()>, //TODO: blend states
    pub write_mask: vk::ColorComponentFlags,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct FragmentState<'a> {
    pub shader: ShaderStage<'a>,
    pub targets: &'a [ColorTargetState],
}

//TODO: StencilState
pub struct RasterPipelineDescription<'a> {
    pub vertex: VertexState<'a>,
    pub primitive: PrimitiveState,
    pub depth_state: Option<DepthState>,
    pub fragment: Option<FragmentState<'a>>,
}

pub(crate) struct RasterPipeline {
    device: Arc<AshDevice>,
    pub handle: vk::Pipeline,
}

impl RasterPipeline {
    pub fn new(
        device: Arc<AshDevice>,
        pipeline_layout: vk::PipelineLayout,
        pipeline_description: &RasterPipelineDescription,
    ) -> Result<Self, VulkanError> {
        let vertex_shader_module = unsafe {
            device.core.create_shader_module(
                &vk::ShaderModuleCreateInfo::builder()
                    .code(pipeline_description.vertex.shader.code),
                None,
            )
        }?;
        let fragment_shader_module = if let Some(fragment_state) = &pipeline_description.fragment {
            Some(unsafe {
                device.core.create_shader_module(
                    &vk::ShaderModuleCreateInfo::builder().code(fragment_state.shader.code),
                    None,
                )
            }?)
        } else {
            None
        };

        let vertex_entry_point_name =
            std::ffi::CString::new(pipeline_description.vertex.shader.entry).unwrap();

        let mut shader_stages = vec![vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_shader_module)
            .name(&vertex_entry_point_name)
            .build()];

        //Keep around to guarantee lifetime
        let _fragment_entry_point_name =
            if let Some(fragment_shader_module) = fragment_shader_module {
                let fragment_entry_point_name = std::ffi::CString::new(
                    pipeline_description.fragment.as_ref().unwrap().shader.entry,
                )
                .unwrap();

                shader_stages.push(
                    vk::PipelineShaderStageCreateInfo::builder()
                        .stage(vk::ShaderStageFlags::FRAGMENT)
                        .module(fragment_shader_module)
                        .name(&fragment_entry_point_name)
                        .build(),
                );
                Some(fragment_entry_point_name)
            } else {
                None
            };

        //TODO: allow config
        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
            .build();

        let mut vertex_binding_descriptions =
            Vec::with_capacity(pipeline_description.vertex.layouts.len());
        let mut vertex_attribute_descriptions = Vec::new();
        for (i, buffer_layout) in pipeline_description.vertex.layouts.iter().enumerate() {
            let i = i as u32;

            vertex_binding_descriptions.push(
                vk::VertexInputBindingDescription::builder()
                    .binding(i)
                    .stride(buffer_layout.stride)
                    .input_rate(buffer_layout.input_rate)
                    .build(),
            );

            for vertex_attribute in buffer_layout.attributes {
                vertex_attribute_descriptions.push(
                    vk::VertexInputAttributeDescription::builder()
                        .binding(i)
                        .location(vertex_attribute.shader_location)
                        .format(vertex_attribute.format)
                        .offset(vertex_attribute.offset)
                        .build(),
                );
            }
        }

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_binding_descriptions)
            .vertex_attribute_descriptions(&vertex_attribute_descriptions);

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

        //TODO: allow config of stencil
        let mut depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder();
        if let Some(depth_state) = &pipeline_description.depth_state {
            depth_stencil_state = depth_stencil_state
                .depth_test_enable(depth_state.depth_enabled)
                .depth_write_enable(depth_state.write_depth)
                .depth_compare_op(depth_state.depth_op)
                .depth_bounds_test_enable(false)
                .min_depth_bounds(0.0)
                .max_depth_bounds(1.0)
        }

        let mut color_attachments_formats: Vec<vk::Format> = Vec::new();
        let mut color_attachments_blend_states: Vec<vk::PipelineColorBlendAttachmentState> =
            Vec::new();
        if let Some(fragment_state) = &pipeline_description.fragment {
            for color_target in fragment_state.targets {
                color_attachments_formats.push(color_target.format);
                color_attachments_blend_states.push(
                    vk::PipelineColorBlendAttachmentState::builder()
                        .color_write_mask(color_target.write_mask)
                        .blend_enable(false)
                        .build(),
                );
            }
        }

        let color_blending_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .attachments(&color_attachments_blend_states)
            .build();

        //TODO: allow depth bias (and other?) dynamic state
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&dynamic_states)
            .build();

        let mut dynamic_rendering = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(&color_attachments_formats);
        if let Some(depth_state) = &pipeline_description.depth_state {
            dynamic_rendering = dynamic_rendering.depth_attachment_format(depth_state.format);
        }
        //TODO: stencil
        // if let Some(stencil_format) = pipeline_description.framebuffer.stencil_attachment {
        //     dynamic_rendering = dynamic_rendering.stencil_attachment_format(stencil_format);
        // }

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

        let result = match unsafe {
            device.core.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_create_info.build()],
                None,
            )
        } {
            Ok(pipelines) => Ok(Self {
                device: device.clone(),
                handle: pipelines[0],
            }),
            Err((_, err)) => return Err(err.into()),
        };

        unsafe {
            device
                .core
                .destroy_shader_module(vertex_shader_module, None);
            if let Some(fragment_shader_module) = fragment_shader_module {
                device
                    .core
                    .destroy_shader_module(fragment_shader_module, None);
            }
        }

        result
    }
}

impl Drop for RasterPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.core.destroy_pipeline(self.handle, None);
        }
    }
}

pub(crate) struct Pipelines {
    device: Arc<AshDevice>,
    pub(crate) layout: vk::PipelineLayout,
    pub(crate) compute: SlotMap<ComputePipelineKey, ComputePipeline>,
    pub(crate) raster: SlotMap<RasterPipleineKey, RasterPipeline>,
}

impl Pipelines {
    pub fn new(device: Arc<AshDevice>, layout: vk::PipelineLayout) -> Self {
        Self {
            device,
            layout,
            compute: SlotMap::with_key(),
            raster: SlotMap::with_key(),
        }
    }
}

impl Drop for Pipelines {
    fn drop(&mut self) {
        self.compute.clear();
        self.raster.clear();

        unsafe {
            self.device.core.destroy_pipeline_layout(self.layout, None);
        }
    }
}
