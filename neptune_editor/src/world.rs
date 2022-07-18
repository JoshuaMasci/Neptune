use crate::entity::Entity;
use crate::physics_world::PhysicsWorld;

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
    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    pub fn update(&mut self, delta_time: f32) {
        for entity in self.entities.iter_mut() {
            entity.update(delta_time);
        }
        self.physics.step(delta_time);
    }
}
