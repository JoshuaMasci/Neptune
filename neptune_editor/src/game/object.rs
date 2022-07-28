use crate::game::Transform;
use crate::physics_world::Collider;
use crate::renderer::Mesh;
use std::sync::Arc;

pub struct Object {
    pub transform: Transform,
    pub mesh: Option<Arc<Mesh>>,
    pub collider: Option<Collider>,
}
