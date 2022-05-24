//TODO: Better blending and stencil settings
#[derive(Hash, Eq, PartialEq)]
pub struct PipelineState {
    pub cull_mode: CullMode,
    pub depth_mode: DepthTestMode,
    pub depth_op: DepthTestOp,

    pub src_factor: BlendFactor,
    pub dst_factor: BlendFactor,
    pub blend_op: BlendOp,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub enum CullMode {
    None,
    Front,
    Back,
    All,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub enum DepthTestMode {
    None,
    TestOnly,
    TestAndWrite,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
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

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
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

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub enum BlendOp {
    None,
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub enum VertexElement {
    Float,
    Float2,
    Float3,
    Float4,
}

impl VertexElement {
    pub fn get_size_bytes(&self) -> u32 {
        let float_size = std::mem::size_of::<f32>() as u32;
        match self {
            VertexElement::Float => float_size,
            VertexElement::Float2 => float_size * 2,
            VertexElement::Float3 => float_size * 3,
            VertexElement::Float4 => float_size * 4,
        }
    }
}
