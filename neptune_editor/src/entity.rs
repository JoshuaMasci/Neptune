use crate::renderer::Mesh;
use crate::transform::Transform;
use crate::world::World;
use std::sync::Arc;

pub trait EntityBehavior {
    fn add_to_world(&mut self, data: &mut EntityInterface);
    fn remove_from_world(&mut self, data: &mut EntityInterface);
    fn update(&mut self, delta_time: f32, data: &mut EntityInterface);
}

pub struct EntityInterface {
    pub(crate) has_transform_changed: bool,
    transform: Transform,

    pub meshes: Vec<(Transform, Arc<Mesh>)>, //TODO: rework this later
}

impl EntityInterface {
    pub fn get_transform(&self) -> Transform {
        self.transform.clone()
    }

    pub fn set_transform(&mut self, transform: Transform) {
        self.has_transform_changed = true;
        self.transform = transform;
    }
}

pub struct Entity {
    pub(crate) entity_id: u64, //TODO: create type for this
    pub(crate) behavior: Box<dyn EntityBehavior>,
    pub(crate) interface: EntityInterface,
}

impl Entity {
    pub fn new<T: EntityBehavior + 'static>(transform: Transform, behavior: T) -> Self {
        Self {
            entity_id: 0,
            behavior: Box::new(behavior),
            interface: EntityInterface {
                has_transform_changed: true,
                transform,

                meshes: vec![],
            },
        }
    }

    pub fn get_transform(&self) -> &Transform {
        &self.interface.transform
    }

    pub fn get_meshes(&self) -> &Vec<(Transform, Arc<Mesh>)> {
        &self.interface.meshes
    }
}

pub fn entity_test() {}
