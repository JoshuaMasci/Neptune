use crate::physics_world::PhysicsWorld;
use crate::renderer::Mesh;
use crate::transform::Transform;
use std::sync::Arc;

pub struct World {
    pub physics: PhysicsWorld,
    pub entities: Vec<Entity>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            physics: PhysicsWorld::new(),
            entities: Vec::new(),
        }
    }
}

impl World {
    pub fn update(&mut self, delta_time: f32) {
        self.physics.step(delta_time);
    }
}

pub struct Entity {
    pub(crate) transform: Transform,
    pub(crate) mesh: Arc<Mesh>,
}
