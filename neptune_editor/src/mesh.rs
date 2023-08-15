pub struct BoundingBox {
    pub min: glam::Vec3,
    pub max: glam::Vec3,
}

pub struct IndexBuffer {
    pub buffer: neptune_vulkan::BufferHandle,
    pub count: u32,
}

pub struct SkinningBuffers {
    pub joint_buffer: neptune_vulkan::BufferHandle,
    pub weight_buffer: neptune_vulkan::BufferHandle,
}

pub struct Primitive {
    pub bounding_box: BoundingBox,

    pub vertex_count: usize,
    pub position_buffer: neptune_vulkan::BufferHandle,

    pub normal_buffer: Option<neptune_vulkan::BufferHandle>,
    pub tangent_buffer: Option<neptune_vulkan::BufferHandle>,

    pub tex_coord_buffers: Vec<neptune_vulkan::BufferHandle>,
    pub color_buffers: Vec<neptune_vulkan::BufferHandle>,
    pub skinning_buffers: Vec<SkinningBuffers>,
    pub index_buffer: Option<IndexBuffer>,
}

#[derive(Default)]
pub struct Mesh {
    pub name: String,
    pub primitives: Vec<Primitive>,
}
