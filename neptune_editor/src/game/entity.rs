use crate::game::world::WorldData;
use crate::input::{ButtonState, InputEventReceiver, StaticString};
use crate::physics::physics_world::Collider;
use crate::scene::scene_renderer::{Model, SceneInstanceHandle};
use crate::transform::Transform;
use glam::{EulerRot, Quat, Vec2, Vec3};
use rapier3d::geometry::ColliderHandle;

//TODO: use this to abstract entity types?
// pub enum EntityType {
//     Player(Player),
//     StaticEntity(StaticEntity),
// }

pub trait Entity {
    fn add_to_world(&mut self, world_data: &mut WorldData);
    fn remove_from_world(&mut self, world_data: &mut WorldData);
    fn update(&mut self, delta_time: f32, world_data: &mut WorldData);
}

//TODO: entities will need a UUID at some point
pub struct StaticEntity {
    // Definition
    transform: Transform,
    model: Model,
    collider: Option<Collider>,

    // World Values
    scene_instance: Option<SceneInstanceHandle>,
    collider_handle: Option<ColliderHandle>,
}

impl StaticEntity {
    pub fn new(transform: Transform, model: Model, collider: Option<Collider>) -> Self {
        Self {
            transform,
            model,
            collider,
            scene_instance: None,
            collider_handle: None,
        }
    }
}

impl Entity for StaticEntity {
    fn add_to_world(&mut self, world_data: &mut WorldData) {
        self.scene_instance = world_data
            .scene
            .add_instance(self.transform.clone(), self.model.clone());

        if let Some(collider) = &self.collider {
            self.collider_handle = Some(world_data.physics.add_collider(
                None,
                &self.transform,
                collider,
            ));
        }
    }

    fn remove_from_world(&mut self, world_data: &mut WorldData) {
        if let Some(scene_instance) = self.scene_instance.take() {
            world_data.scene.remove_instance(scene_instance);
        }

        if let Some(collider_handle) = self.collider_handle.take() {
            world_data.physics.remove_collider(collider_handle)
        }
    }

    fn update(&mut self, delta_time: f32, world_data: &mut WorldData) {
        if let Some(scene_instance) = self.scene_instance {
            world_data
                .scene
                .update_instance(scene_instance, self.transform.clone());
        }

        if let Some(collider_handle) = &self.collider_handle {
            world_data
                .physics
                .update_collider_transform(*collider_handle, &self.transform);
        }
    }
}
