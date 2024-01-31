use crate::game::world::WorldData;
use crate::scene::scene_renderer::SceneInstanceHandle;
use crate::transform::Transform;
use crate::{InputSystem, Model};
use glam::{Vec2, Vec3};

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

pub struct Player {
    transform: Transform,

    /// units: rad
    pitch_yaw: Vec2,

    // Properties
    /// units: m/s
    linear_speed: Vec3,

    /// pitch yaw: rad/s
    angular_speed: Vec2,

    // Input
    linear_input: Vec3,
    angular_input: Vec2,
}

impl Player {
    pub fn with_position(position: Vec3) -> Self {
        Self {
            transform: Transform::with_position(position),
            pitch_yaw: Vec2::ZERO,
            linear_speed: Vec3::splat(0.5),
            angular_speed: Vec2::splat(std::f32::consts::PI),
            linear_input: Vec3::ZERO,
            angular_input: Vec2::ZERO,
        }
    }

    pub fn process_input(&mut self, input_system: &mut InputSystem) {}
}

impl Entity for Player {
    fn add_to_world(&mut self, world_data: &mut WorldData) {
        todo!()
    }

    fn remove_from_world(&mut self, world_data: &mut WorldData) {
        todo!()
    }

    fn update(&mut self, delta_time: f32, world_data: &mut WorldData) {
        self.pitch_yaw += self.angular_input * self.angular_speed;

        //Clamp pitch 180 deg arc
        const PI_2: f32 = std::f32::consts::FRAC_PI_2;
        self.pitch_yaw.x = self.pitch_yaw.x.clamp(-PI_2, PI_2);

        self.transform.position +=
            self.transform.rotation * (self.linear_input * self.linear_speed);
    }
}

//TODO: entities will need a UUID at some point
pub struct StaticEntity {
    // Definition
    transform: Transform,
    model: Model,

    // World Values
    scene_instance: Option<SceneInstanceHandle>,
}

impl StaticEntity {
    pub fn new(transform: Transform, model: Model) -> Self {
        Self {
            transform,
            model,
            scene_instance: None,
        }
    }
}

impl Entity for StaticEntity {
    fn add_to_world(&mut self, world_data: &mut WorldData) {
        self.scene_instance = world_data.scene.add_instance(
            self.transform.clone(),
            self.model.mesh.clone(),
            self.model.material.clone(),
        );
    }

    fn remove_from_world(&mut self, world_data: &mut WorldData) {
        if let Some(scene_instance) = self.scene_instance.take() {
            world_data.scene.remove_instance(scene_instance);
        }
    }

    fn update(&mut self, delta_time: f32, world_data: &mut WorldData) {
        if let Some(scene_instance) = self.scene_instance {
            world_data
                .scene
                .update_instance(scene_instance, self.transform.clone());
        }
    }
}
