use crate::game::physics_world::Collider;
use crate::game::Transform;
use crate::rendering::mesh::Mesh;
use std::sync::Arc;

pub struct Object {
    pub transform: Transform,
    pub mesh: Option<Arc<Mesh>>,
    pub collider: Option<Collider>,
}
