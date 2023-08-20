#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct VertexAttributes {
    pub normal: glam::Vec3,
    pub tangent: glam::Vec4,
    pub tex_coords: glam::Vec4,
    pub color: glam::Vec4,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct VertexSkinningAttributes {
    pub joint: glam::UVec4,
    pub weight: glam::Vec4,
}

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
