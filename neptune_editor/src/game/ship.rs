use crate::game::entity::Entity;
use crate::game::world::WorldData;
use crate::physics::physics_world::Collider;
use crate::scene::scene_renderer::{Model, SceneInstanceHandle};
use crate::transform::Transform;
use rapier3d::dynamics::RigidBodyHandle;
use rapier3d::geometry::ColliderHandle;

pub enum ModuleType {
    Connector,
    Hallway,
    Room,
}

#[derive(Clone)]
pub struct Module {
    pub model: Model,
    pub collider: Collider,
}

pub struct ModuleInstance {
    transform: Transform,
    model_handle: SceneInstanceHandle,
    collider_handle: ColliderHandle,
}

pub struct Ship {
    pub connector_module: Module,
    pub hallway_module: Module,
    pub room_module: Module,

    pub module_list: Vec<(Transform, ModuleType)>,

    pub transform: Transform,
    pub rigid_body_handle: Option<RigidBodyHandle>,
    pub modules: Vec<ModuleInstance>,
}

impl Entity for Ship {
    fn add_to_world(&mut self, world_data: &mut WorldData) {
        let rigid_body_handle = world_data.physics.add_rigid_body(&self.transform);
        self.rigid_body_handle = Some(rigid_body_handle);

        self.modules = self
            .module_list
            .iter()
            .map(|(transform, module_type)| {
                let module = match module_type {
                    ModuleType::Connector => &self.connector_module,
                    ModuleType::Hallway => &self.hallway_module,
                    ModuleType::Room => &self.room_module,
                };

                ModuleInstance {
                    transform: transform.clone(),
                    model_handle: world_data
                        .scene
                        .add_instance(transform.clone(), module.model.clone())
                        .unwrap(),
                    collider_handle: world_data.physics.add_collider(
                        self.rigid_body_handle,
                        transform,
                        &module.collider,
                    ),
                }
            })
            .collect()
    }

    fn remove_from_world(&mut self, world_data: &mut WorldData) {
        let _ = world_data;
    }

    fn update(&mut self, delta_time: f32, world_data: &mut WorldData) {
        let _ = delta_time;
        let _ = world_data;

        if let Some(rigid_body_ref) = world_data
            .physics
            .get_mut_rigid_body(self.rigid_body_handle)
        {
            rigid_body_ref.get_transform(&mut self.transform);

            for module in self.modules.iter() {
                world_data.scene.update_instance(
                    module.model_handle,
                    self.transform.transform(&module.transform),
                );
            }
        }
    }
}
