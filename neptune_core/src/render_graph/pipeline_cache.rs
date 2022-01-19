use ash::vk;
use std::collections::HashMap;

// pub struct PipelineCache {
//     graphics_pipelines: HashMap<GraphicsPipelineDescription, vk::Pipeline>,
// }
//
// impl PipelineCache {
//     pub fn get_graphics(
//         &mut self,
//         pipeline_description: &GraphicsPipelineDescription,
//     ) -> vk::Pipeline {
//         if let Some(&pipeline) = self.graphics_pipelines.get(pipeline_description) {
//             pipeline
//         } else {
//             //TODO: build piplines
//             vk::Pipeline::null()
//         }
//     }
// }

pub enum CullMode {
    None,
    Front,
    Back,
    All,
}

pub enum DepthTestMode {
    None,
    TestOnly,
    TestAndWrite,
}

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

enum BlendFactor {
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

enum BlendOp {
    None,
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

//TODO: rework this
enum VertexElement {
    float,
    float2,
    float3,
    float4,
}

pub struct GraphicsPipelineDescription {
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
    cull_mode: CullMode,
    depth_mode: DepthTestMode,
    depth_op: DepthTestOp,
    src_factor: BlendFactor,
    dst_factor: BlendFactor,
    blend_op: BlendOp,
    vertex_elements: Vec<VertexElement>,
}
