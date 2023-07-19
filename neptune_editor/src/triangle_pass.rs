use glam::Vec3;
use neptune_vulkan::gpu_allocator::MemoryLocation;
use neptune_vulkan::{vk, AshBuffer, AshDevice, BufferResource, PersistentResourceManager};
use std::collections::HashMap;
use std::sync::Arc;

pub struct TrianglePass {
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,

    vertex_buffer: BufferResource,
    index_buffer: BufferResource,
}

fn write_data<T>(ptr: &mut [u8], data: &[T]) {
    let data_slice = unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data))
    };
    ptr[0..data_slice.len()].copy_from_slice(data_slice);
}

impl TrianglePass {
    pub fn new(
        device: Arc<AshDevice>,
        resource_manager: &mut PersistentResourceManager,
        color_format: vk::Format,
        depth_stencil_format: vk::Format,
    ) -> Self {
        let vertex_data = [
            Vec3::new(0.25, 0.25, 0.5),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 0.75, 0.5),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.75, 0.25, 0.5),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        let index_data = [0, 1, 2];

        let mut vertex_buffer = AshBuffer::new(
            &device,
            &vk::BufferCreateInfo::builder()
                .size(std::mem::size_of_val(&vertex_data) as vk::DeviceSize)
                .usage(vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER),
            MemoryLocation::CpuToGpu,
        );
        let mut index_buffer = AshBuffer::new(
            &device,
            &vk::BufferCreateInfo::builder()
                .size(std::mem::size_of_val(&index_data) as vk::DeviceSize)
                .usage(vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER),
            MemoryLocation::CpuToGpu,
        );

        let vertex_ptr = vertex_buffer
            .allocation
            .mapped_slice_mut()
            .expect("Failed to read map buffer");
        write_data(vertex_ptr, &vertex_data);

        let index_ptr = index_buffer
            .allocation
            .mapped_slice_mut()
            .expect("Failed to read map buffer");
        write_data(index_ptr, &index_data);

        let vertex_buffer = BufferResource::Persistent(resource_manager.add_buffer(vertex_buffer));
        let index_buffer = BufferResource::Persistent(resource_manager.add_buffer(index_buffer));

        let pipeline_layout = unsafe {
            device
                .core
                .create_pipeline_layout(&vk::PipelineLayoutCreateInfo::builder(), None)
                .expect("Failed to create pipeline layout")
        };

        Self {
            pipeline_layout,
            pipeline: create_pipeline(&device, color_format, depth_stencil_format, pipeline_layout),
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn build_render_graph(
        &self,
        render_graph: &mut neptune_vulkan::RenderGraph,
        render_target: neptune_vulkan::ImageResource,
    ) {
        // let depth_image = render_graph.create_transient_image(neptune_vulkan::TransientImageDesc {
        //     size: neptune_vulkan::TransientImageSize::Relative([1.0; 2], render_target),
        //     format: vk::Format::D32_SFLOAT,
        //     mip_levels: 1,
        //     memory_location: MemoryLocation::GpuOnly,
        // });

        let mut image_usages = HashMap::new();
        image_usages.insert(
            render_target,
            neptune_vulkan::ImageAccess {
                write: true,
                stage: vk::PipelineStageFlags2::FRAGMENT_SHADER,
                access: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                layout: vk::ImageLayout::ATTACHMENT_OPTIMAL,
            },
        );
        // image_usages.insert(
        //     depth_image,
        //     neptune_vulkan::ImageAccess {
        //         write: true,
        //         stage: vk::PipelineStageFlags2::PRE_RASTERIZATION_SHADERS,
        //         access: vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
        //         layout: vk::ImageLayout::ATTACHMENT_OPTIMAL,
        //     },
        //);

        let pipeline = self.pipeline;
        let vertex_buffer_handle = self.vertex_buffer;
        let index_buffer_handle = self.index_buffer;

        let build_cmd_fn: Box<neptune_vulkan::BuildCommandFn> =
            Box::new(move |device, command_buffer, resources| unsafe {
                let vertex_buffer = resources.get_buffer(vertex_buffer_handle);
                let index_buffer = resources.get_buffer(index_buffer_handle);

                device.core.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline,
                );

                device.core.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[vertex_buffer.handle],
                    &[0],
                );
                device.core.cmd_bind_index_buffer(
                    command_buffer,
                    index_buffer.handle,
                    0,
                    vk::IndexType::UINT32,
                );

                device.core.cmd_draw_indexed(command_buffer, 3, 1, 0, 0, 0);
            });

        let basic_render_pass = neptune_vulkan::RenderPass {
            name: String::from("Basic Render Pass"),
            queue: vk::Queue::null(), //TODO: this
            buffer_usages: Default::default(),
            image_usages,
            framebuffer: Some(neptune_vulkan::Framebuffer {
                color_attachments: vec![neptune_vulkan::ColorAttachment::new_clear(
                    render_target,
                    [0.29, 0.0, 0.5, 0.0],
                )],
                depth_stencil_attachment: None,
                input_attachments: vec![],
            }),
            build_cmd_fn: Some(build_cmd_fn),
        };
        render_graph.add_pass(basic_render_pass);
    }
}

fn create_pipeline(
    device: &AshDevice,
    color_format: vk::Format,
    depth_stencil_format: vk::Format,
    pipeline_layout: vk::PipelineLayout,
) -> vk::Pipeline {
    let entry_point_name = std::ffi::CString::new("main").unwrap();

    let vertex_code = include_bytes!("../resource/shader/triangle.vert.spv");
    let fragment_code = include_bytes!("../resource/shader/triangle.frag.spv");

    let vertex_shader = unsafe {
        device.core.create_shader_module(
            &vk::ShaderModuleCreateInfo::builder().code(std::slice::from_raw_parts(
                vertex_code.as_ptr() as *const u32,
                vertex_code.len() / 4,
            )),
            None,
        )
    }
    .unwrap();

    let fragment_shader = unsafe {
        device.core.create_shader_module(
            &vk::ShaderModuleCreateInfo::builder().code(std::slice::from_raw_parts(
                fragment_code.as_ptr() as *const u32,
                fragment_code.len() / 4,
            )),
            None,
        )
    }
    .unwrap();

    let shader_states_infos = [
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_shader)
            .name(&entry_point_name)
            .build(),
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_shader)
            .name(&entry_point_name)
            .build(),
    ];

    let attribute_description = [
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build(),
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(std::mem::size_of::<Vec3>() as u32)
            .build(),
    ];

    let binding_description = [vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(std::mem::size_of::<Vec3>() as u32 * 2)
        .input_rate(vk::VertexInputRate::VERTEX)
        .build()];

    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(&binding_description)
        .vertex_attribute_descriptions(&attribute_description);

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
        .blend_enable(false)
        .build()];

    let color_blending_info = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .logic_op(vk::LogicOp::COPY)
        .attachments(&color_blend_attachments)
        .blend_constants([0.0, 0.0, 0.0, 0.0]);

    let dynamic_states = [vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT];
    let dynamic_states_info =
        vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

    let color_attachment_formats = [color_format];
    let mut dynamic_rendering_info = vk::PipelineRenderingCreateInfoKHR::builder()
        .view_mask(0)
        .color_attachment_formats(&color_attachment_formats);

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
        .render_pass(vk::RenderPass::null())
        .subpass(0)
        .push_next(&mut dynamic_rendering_info)
        .build()];

    let pipeline = unsafe {
        device
            .core
            .create_graphics_pipelines(vk::PipelineCache::null(), pipeline_info, None)
    }
    .expect("Failed to create pipeline")[0];

    unsafe {
        device.core.destroy_shader_module(vertex_shader, None);
        device.core.destroy_shader_module(fragment_shader, None);
    }

    pipeline
}
