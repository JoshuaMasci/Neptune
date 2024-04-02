use crate::universe::entity::EntityData;
use crate::universe::world::World;
use std::any::{Any, TypeId};
use std::collections::HashMap;

pub trait EntitySystem {
    fn update_pre_physics(&mut self, entity: &mut EntityData, delta_time: f32);
    fn update_post_physics(&mut self, entity: &mut EntityData, delta_time: f32);

    fn add_to_world(&mut self, world: &mut World, entity: &mut EntityData);
    fn remove_from_world(&mut self, world: &mut World, entity: &mut EntityData);
}

#[derive(Default)]
pub struct EntitySystemPool {
    system_map: HashMap<TypeId, Box<dyn Any>>,
}

impl EntitySystemPool {
    pub fn insert<T: EntitySystem + 'static>(&mut self, system: T) -> Option<T> {
        self.system_map
            .insert(TypeId::of::<T>(), Box::new(system))
            .map(|boxed| *boxed.downcast().unwrap())
    }

    pub fn remove<T: EntitySystem + 'static>(&mut self) -> Option<T> {
        self.system_map
            .remove(&TypeId::of::<T>())
            .map(|boxed| *boxed.downcast().unwrap())
    }

    pub fn get<T: EntitySystem + 'static>(&self) -> Option<&T> {
        self.system_map
            .get(&TypeId::of::<T>())
            .map(|boxed| boxed.downcast_ref().unwrap())
    }

    pub fn get_mut<T: EntitySystem + 'static>(&mut self) -> Option<&mut T> {
        self.system_map
            .get_mut(&TypeId::of::<T>())
            .map(|boxed| boxed.downcast_mut().unwrap())
    }
}

impl EntitySystem for EntitySystemPool {
    fn update_pre_physics(&mut self, entity: &mut EntityData, delta_time: f32) {
        for (_type_id, system) in self.system_map.iter_mut() {
            if let Some(system) = system.downcast_mut::<&mut dyn EntitySystem>() {
                system.update_pre_physics(entity, delta_time);
            }
        }
    }

    fn update_post_physics(&mut self, entity: &mut EntityData, delta_time: f32) {
        for (_type_id, system) in self.system_map.iter_mut() {
            if let Some(system) = system.downcast_mut::<&mut dyn EntitySystem>() {
                system.update_post_physics(entity, delta_time);
            }
        }
    }

    fn add_to_world(&mut self, world: &mut World, entity: &mut EntityData) {
        for (_type_id, system) in self.system_map.iter_mut() {
            if let Some(system) = system.downcast_mut::<&mut dyn EntitySystem>() {
                system.add_to_world(world, entity);
            }
        }
    }

    fn remove_from_world(&mut self, world: &mut World, entity: &mut EntityData) {
        for (_type_id, system) in self.system_map.iter_mut() {
            if let Some(system) = system.downcast_mut::<&mut dyn EntitySystem>() {
                system.remove_from_world(world, entity);
            }
        }
    }
}

#[macro_export]
macro_rules! systems {
    [] => (
       EntitySystemPool::default()
    );
    [$($system:expr),*] => {{
        let mut pool = EntitySystemPool::default();
        $(pool.insert($system);)*
        pool
    }};
}
