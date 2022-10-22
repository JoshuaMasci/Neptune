use crate::game::physics_world::Collider;
use crate::game::Transform;
use crate::rendering::material::Material;
use crate::rendering::mesh::Mesh;
use std::sync::Arc;

pub struct Model {
    pub mesh: Arc<Mesh>,
    pub material: Arc<Material>,
}

pub struct Object {
    pub transform: Transform,
    pub model: Option<Model>,
    pub collider: Option<Collider>,
}
