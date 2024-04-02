use crate::transform::Transform;
use crate::universe::entity::{Entity, EntityData, Node};
use glam::Vec3;

pub trait WorldSystem {
    fn update_pre_physics(&mut self, world: &mut World, delta_time: f32);
    fn update_pre_post(&mut self, world: &mut World, delta_time: f32);
}

#[derive(Default)]
pub struct World {
    entities: Vec<Entity>,
}

impl World {
    pub fn add_to_world(&mut self, mut entity: Entity) {
        entity.systems.add_to_world(self, &mut entity.data);
        self.entities.push(entity);
    }
}

use crate::physics::physics_world::Collider;
use crate::universe::components::{
    CharacterComponent, ColliderComponent, ModelComponent, RigidBodyComponent,
};
use crate::universe::entity::ComponentPool;
use crate::universe::system::{EntitySystem, EntitySystemPool};
use crate::{components, systems};

pub fn init_test_world() -> World {
    //TODO: load world from file

    let purple_cube_model = ModelComponent {
        mesh_name: "Cube".to_string(),
        material_names: vec!["Purple".to_string()],
    };

    let orange_cube_model = ModelComponent {
        mesh_name: "Cube".to_string(),
        material_names: vec!["Orange".to_string()],
    };

    let mut world = World::default();

    // Level Entity
    {
        let mut level_entity = EntityData {
            name: "Level".to_string(),
            transform: Transform::with_position(Vec3::new(0.0, -0.5, 0.0)),
            ..Default::default()
        };

        let ground_scale = Vec3::new(8.0, 0.5, 8.0);
        let platform_collider = ColliderComponent {
            mass: 0.0, // Will be static so mass should be zero
            collider: Collider::Box(ground_scale),
        };

        for i in 0..3 {
            let _ = level_entity.nodes.insert(
                None,
                Node {
                    name: "Platform".to_string(),
                    local_transform: Transform {
                        position: Vec3::new(0.0, 0.0, i as f32 * ground_scale.z),
                        scale: ground_scale,
                        ..Default::default()
                    },
                    components: components![orange_cube_model.clone(), platform_collider.clone()],
                    ..Default::default()
                },
            );
        }

        world.add_to_world(Entity {
            data: level_entity,
            systems: systems![],
        });
    }

    // Ship Entity
    {
        let mut ship_entity = EntityData {
            name: "Ship".to_string(),
            transform: Transform::with_position(Vec3::Y * 7.0 + Vec3::Z * 2.0),
            components: components![RigidBodyComponent::default()],
            ..Default::default()
        };

        {
            let module_collider = ColliderComponent {
                mass: 100.0,
                collider: Collider::Box(Vec3::ONE),
            };
            let _ = ship_entity.nodes.insert(
                None,
                Node {
                    name: "Module".to_string(),
                    local_transform: Transform::with_position(Vec3::new(0.0, 2.0, 0.0)),
                    components: components![purple_cube_model.clone(), module_collider.clone()],
                    ..Default::default()
                },
            );
            let _ = ship_entity.nodes.insert(
                None,
                Node {
                    name: "Module".to_string(),
                    local_transform: Transform::with_position(Vec3::new(0.0, 0.0, 0.0)),
                    components: components![purple_cube_model.clone(), module_collider.clone()],
                    ..Default::default()
                },
            );
            let _ = ship_entity.nodes.insert(
                None,
                Node {
                    name: "Module".to_string(),
                    local_transform: Transform::with_position(Vec3::new(0.0, -2.0, 0.0)),
                    components: components![purple_cube_model.clone(), module_collider.clone()],
                    ..Default::default()
                },
            );
        }

        world.add_to_world(Entity {
            data: ship_entity,
            systems: systems![],
        });
    }

    // Player Entity
    {
        let player_entity = EntityData {
            name: "Player".to_string(),
            transform: Transform::with_position(Vec3::Y * 3.0),
            components: components![
                CharacterComponent::default(),
                ColliderComponent {
                    mass: 75.0,
                    collider: Collider::CapsuleY(1.8, 0.3),
                }
            ],
            ..Default::default()
        };

        world.add_to_world(Entity {
            data: player_entity,
            systems: systems![],
        });
    }

    world
}
