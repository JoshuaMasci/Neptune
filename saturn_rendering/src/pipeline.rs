use ash::*;

pub trait VertexInputDescription {
    fn get_binding_description(&self) -> &'static [vk::VertexInputBindingDescription];
    fn get_attribute_description(&self) -> &'static [vk::VertexInputAttributeDescription];
}

pub struct GraphicsPipelineState {
    cull_mode: vk::CullModeFlags,
}

pub struct GraphicsPipeline {
    device: ash::Device,
    pipeline: vk::Pipeline,
}

impl GraphicsPipeline {
    fn new<T: VertexInputDescription>(
        device: ash::Device,
        vertex_input: &T,
        state: &GraphicsPipelineState,
    ) -> Self {
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(vertex_input.get_binding_description())
            .vertex_attribute_descriptions(vertex_input.get_attribute_description())
            .build();

        let create_info = vk::GraphicsPipelineCreateInfo::builder()
            .vertex_input_state(&vertex_input_state)
            .build();

        let pipeline = unsafe {
            device.create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
        }
        .expect("Failed to create graphics pipeline")[0];

        Self { device, pipeline }
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}
