use crate::universe::entity::Entity;

pub trait WorldSystem {
    fn update_pre_physics(&mut self, world: &mut World, delta_time: f32);
    fn update_pre_post(&mut self, world: &mut World, delta_time: f32);
}

pub struct World {
    entities: Vec<Entity>,
}
