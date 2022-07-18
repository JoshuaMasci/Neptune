use crate::renderer::Mesh;
use crate::transform::Transform;
use std::sync::Arc;

pub trait EntityBehavior {
    fn update(&mut self, delta_time: f32, data: &mut EntityData);
    fn get_mesh(&self) -> Option<Arc<Mesh>>;
}

pub struct EntityData {
    transform: Transform,
}

pub struct Entity {
    entity_id: u64, //TODO: create type for this
    behavior: Box<dyn EntityBehavior>,
    data: EntityData,
}

impl Entity {
    pub fn new<T: EntityBehavior + 'static>(transform: Transform, behavior: T) -> Self {
        Self {
            entity_id: 0,
            behavior: Box::new(behavior),
            data: EntityData { transform },
        }
    }

    pub fn get_transform(&self) -> &Transform {
        &self.data.transform
    }

    pub fn update(&mut self, delta_time: f32) {
        self.behavior.update(delta_time, &mut self.data);
    }

    pub fn get_mesh(&self) -> Option<Arc<Mesh>> {
        self.behavior.get_mesh()
    }
}

struct TestBehavior(f32);
impl EntityBehavior for TestBehavior {
    fn update(&mut self, delta_time: f32, data: &mut EntityData) {
        data.transform.position.z += (delta_time * self.0) as f64;
    }

    fn get_mesh(&self) -> Option<Arc<Mesh>> {
        None
    }
}

pub fn entity_test() {
    let mut new_entity = Entity::new(Transform::default(), TestBehavior(1.0));
    new_entity.update(1.0 / 60.0);
}
