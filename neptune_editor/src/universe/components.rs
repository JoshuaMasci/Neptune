use crate::physics::physics_world::Collider;
use glam::Vec3;

// The current plan for Components (Entity + Node) to be P.O.D. (Plain Old Data) and contain no world specific data
// The Entity Systems attached to each entity will be responsible for all logic as well as registering the entity with the world (as well as world systems)
// Entities therefor should be very easy to serialize and move between worlds

#[derive(Default, Clone)]
pub struct ModelComponent {
    pub mesh_name: String,
    pub material_names: Vec<String>,
}

#[derive(Clone)]
pub struct ColliderComponent {
    pub mass: f32,
    pub collider: Collider,
}

#[derive(Default, Clone)]
pub struct RigidBodyComponent {
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
}

#[derive(Default, Clone)]
pub struct CharacterComponent {}
