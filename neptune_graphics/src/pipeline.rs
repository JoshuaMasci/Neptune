#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum IndexSize {
    U16,
    U32,
}

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

impl PipelineState {
    pub fn alpha_blending_basic() -> Self {
        Self {
            cull_mode: CullMode::None,
            depth_mode: DepthTestMode::None,
            depth_op: DepthTestOp::Never,
            src_factor: BlendFactor::AlphaSrc,
            dst_factor: BlendFactor::OneMinusAlphaSrc,
            blend_op: BlendOp::Add,
        }
    }
}

impl Default for PipelineState {
    fn default() -> Self {
        Self {
            cull_mode: CullMode::None,
            depth_mode: DepthTestMode::None,
            depth_op: DepthTestOp::Never,
            src_factor: BlendFactor::Zero,
            dst_factor: BlendFactor::Zero,
            blend_op: BlendOp::None,
        }
    }
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

//TODO: Rename these elements
#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub enum VertexElement {
    Byte,
    Byte2,
    Byte3,
    Byte4,
    Float,
    Float2,
    Float3,
    Float4,
}

impl VertexElement {
    pub fn get_size_bytes(&self) -> u32 {
        const BYTE_SIZE: u32 = std::mem::size_of::<u8>() as u32;
        const FLOAT_SIZE: u32 = std::mem::size_of::<f32>() as u32;
        match self {
            VertexElement::Byte => BYTE_SIZE,
            VertexElement::Byte2 => BYTE_SIZE * 2,
            VertexElement::Byte3 => BYTE_SIZE * 3,
            VertexElement::Byte4 => BYTE_SIZE * 4,
            VertexElement::Float => FLOAT_SIZE,
            VertexElement::Float2 => FLOAT_SIZE * 2,
            VertexElement::Float3 => FLOAT_SIZE * 3,
            VertexElement::Float4 => FLOAT_SIZE * 4,
        }
    }
}
