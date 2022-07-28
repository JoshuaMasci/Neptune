use crate::entity::{Entity, EntityId};
use crate::physics_world::PhysicsWorld;
use crate::transform::Transform;

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
    pub fn add_entity(&mut self, mut entity: Entity) {
        entity.add_to_world(self);
        self.entities.push(entity);
    }

    pub fn remove_from_world(&mut self, entity_id: EntityId) {
        if let Some(index) = self
            .entities
            .iter()
            .position(|entity| entity.entity_id == entity_id)
        {
            let mut entity = self.entities.swap_remove(index);
            entity.remove_from_world(self);

            if let Some(rigid_body) = entity.rigid_body {
                self.physics.remove_rigid_body(rigid_body);
            }
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        //Pre physics step
        self.physics.step(delta_time);

        //Post physics step
        for entity in self.entities.iter_mut() {}
    }
}
