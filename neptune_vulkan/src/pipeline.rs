use crate::VulkanError;
use ash::vk;

pub struct FramebufferDesc {
    color_attachments: Vec<vk::Format>,
    depth_stencile_attachment: Option<vk::Format>,
}

pub struct VertexInputDesc {
    format: vk::Format,
    stride: u32,
}

pub struct RasterPipelineDesc<'a> {
    framebuffer: FramebufferDesc,
    vertex_input: VertexInputDesc,
    vertex_shader: &'a [u32],
    fragment_shader: Option<&'a [u32]>,
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
        .module(vertex_shader_module)
        .name(&entry_point_name)
        .build()];

    if let Some(fragment_shader_module) = fragment_shader_module {
        shader_stages.push(
            vk::PipelineShaderStageCreateInfo::builder()
                .module(fragment_shader_module)
                .name(&entry_point_name)
                .build(),
        );
    }

    let input_binding_description = vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(desc.vertex_input.stride)
        .input_rate(vk::VertexInputRate::VERTEX)
        .build();

    let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false)
        .build();

    vk::VertexInputAttributeDescription::builder()
        .binding(0)
        .location(0)
        .format(desc.vertex_input.format)
        .offset(0)
        .build();

    let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&shader_stages)
        .input_assembly_state(&input_assembly_info)
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
