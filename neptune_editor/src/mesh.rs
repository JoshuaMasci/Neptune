use memoffset::offset_of;
use neptune_vulkan::vk;

#[repr(transparent)]
pub struct VertexPosition(glam::Vec3);

impl VertexPosition {
    pub const VERTEX_BUFFER_LAYOUT: neptune_vulkan::VertexBufferLayout<'static> =
        neptune_vulkan::VertexBufferLayout {
            stride: std::mem::size_of::<glam::Vec3>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
            attributes: &[neptune_vulkan::VertexAttribute {
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 0,
                shader_location: 0,
            }],
        };
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct VertexAttributes {
    pub normal: glam::Vec3,
    pub tangent: glam::Vec4,
    pub tex_coords: glam::Vec4,
    pub color: glam::Vec4,
}

impl VertexAttributes {
    pub const VERTEX_BUFFER_LAYOUT: neptune_vulkan::VertexBufferLayout<'static> =
        neptune_vulkan::VertexBufferLayout {
            stride: std::mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
            attributes: &[
                neptune_vulkan::VertexAttribute {
                    shader_location: 1,
                    format: vk::Format::R32G32B32_SFLOAT,
                    offset: offset_of!(Self, normal) as u32,
                },
                neptune_vulkan::VertexAttribute {
                    shader_location: 2,
                    format: vk::Format::R32G32B32A32_SFLOAT,
                    offset: offset_of!(Self, tangent) as u32,
                },
                neptune_vulkan::VertexAttribute {
                    shader_location: 3,
                    format: vk::Format::R32G32B32A32_SFLOAT,
                    offset: offset_of!(Self, tex_coords) as u32,
                },
                neptune_vulkan::VertexAttribute {
                    shader_location: 4,
                    format: vk::Format::R32G32B32A32_SFLOAT,
                    offset: offset_of!(Self, color) as u32,
                },
            ],
        };
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct VertexSkinningAttributes {
    pub joint: glam::UVec4,
    pub weight: glam::Vec4,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct BoundingBox {
    pub min: glam::Vec3,
    pub max: glam::Vec3,
}

pub struct IndexBuffer {
    pub buffer: neptune_vulkan::BufferHandle,
    pub count: u32,
}

pub struct Primitive {
    pub bounding_box: BoundingBox,

    pub vertex_count: usize,
    pub position_buffer: neptune_vulkan::BufferHandle,
    pub attributes_buffer: neptune_vulkan::BufferHandle,
    pub skinning_buffer: Option<neptune_vulkan::BufferHandle>,
    pub index_buffer: Option<IndexBuffer>,
}

#[derive(Default)]
pub struct Mesh {
    pub name: String,
    pub primitives: Vec<Primitive>,
}
