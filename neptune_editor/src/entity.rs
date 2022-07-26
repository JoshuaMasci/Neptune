use crate::physics_world::Collider;
use crate::renderer::Mesh;
use crate::transform::Transform;
use crate::world::World;
use rapier3d_f64::prelude::ColliderHandle;
use rapier3d_f64::prelude::RigidBodyHandle;
use std::sync::Arc;

pub type EntityId = u32;

// pub trait EntityBehavior {
//     fn add_to_world(&mut self, data: &mut EntityInterface);
//     fn remove_from_world(&mut self, data: &mut EntityInterface);
//     fn update(&mut self, delta_time: f32, data: &mut EntityInterface);
// }
//
// pub struct EntityInterface {
//     pub(crate) has_transform_changed: bool,
//     transform: Transform,
//
//     pub meshes: Vec<(Transform, Arc<Mesh>)>, //TODO: rework this later
// }
//
// impl EntityInterface {
//     pub fn get_transform(&self) -> Transform {
//         self.transform.clone()
//     }
//
//     pub fn set_transform(&mut self, transform: Transform) {
//         self.has_transform_changed = true;
//         self.transform = transform;
//     }
// }

//Temp

pub struct Entity {
    pub(crate) entity_id: EntityId,
    transform: Transform,
    meshes: Vec<(Transform, Arc<Mesh>)>,
    pub(crate) collider: Option<Collider>,
    pub(crate) rigid_body: Option<RigidBodyHandle>,
}

impl Entity {
    pub fn new(transform: Transform, mesh: Option<Arc<Mesh>>, collider: Option<Collider>) -> Self {
        let meshes = if let Some(mesh) = mesh {
            vec![(Transform::default(), mesh)]
        } else {
            Vec::new()
        };

        Self {
            entity_id: 0,
            transform,
            meshes,
            collider,
            rigid_body: None,
        }
    }

    pub fn get_transform(&self) -> &Transform {
        &self.transform
    }

    pub fn get_mut_transform(&mut self) -> &mut Transform {
        &mut self.transform
    }

    pub fn get_meshes(&self) -> &Vec<(Transform, Arc<Mesh>)> {
        &self.meshes
    }

    pub fn add_to_world(&mut self, world: &mut World) {}

    pub fn remove_from_world(&mut self, world: &mut World) {}
}
