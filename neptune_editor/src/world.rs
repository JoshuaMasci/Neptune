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
    pub fn add_entity(&mut self, mut entity: Entity) {
        entity.behavior.add_to_world(&mut entity.interface);
        self.entities.push(entity);
    }

    pub fn update(&mut self, delta_time: f32) {
        //Pre physics step

        self.physics.step(delta_time);

        //Post physics step
        for entity in self.entities.iter_mut() {
            entity.behavior.update(delta_time, &mut entity.interface);
            if entity.interface.has_transform_changed {
                //do physics things
                entity.interface.has_transform_changed = false;
            }
        }
    }
}
