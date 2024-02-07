use crate::game::world::WorldData;
use crate::input::{ButtonState, InputEventReceiver, StaticString};
use crate::scene::scene_renderer::{Model, SceneInstanceHandle};
use crate::transform::Transform;
use glam::{EulerRot, Quat, Vec2, Vec3};

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
    pub(crate) transform: Transform,

    up: Vec3,

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
            up: Vec3::Y,
            pitch_yaw: Vec2::ZERO,
            linear_speed: Vec3::splat(1.0),
            angular_speed: Vec2::splat(std::f32::consts::PI),
            linear_input: Vec3::ZERO,
            angular_input: Vec2::ZERO,
        }
    }
}

impl Entity for Player {
    fn add_to_world(&mut self, world_data: &mut WorldData) {}

    fn remove_from_world(&mut self, world_data: &mut WorldData) {}

    fn update(&mut self, delta_time: f32, world_data: &mut WorldData) {
        self.pitch_yaw += self.angular_input * self.angular_speed * delta_time;

        //Clamp pitch 180 deg arc
        const PI_2: f32 = std::f32::consts::FRAC_PI_2;
        self.pitch_yaw.x = self.pitch_yaw.x.clamp(-PI_2, PI_2);

        self.transform.rotation =
            Quat::from_euler(EulerRot::YXZ, self.pitch_yaw.y, self.pitch_yaw.x, 0.0);

        self.transform.position +=
            self.transform.rotation * (self.linear_input * self.linear_speed * delta_time);
    }
}

impl InputEventReceiver for Player {
    fn requests_mouse_capture(&mut self) -> bool {
        todo!()
    }

    fn on_button_event(&mut self, button_name: StaticString, state: ButtonState) -> bool {
        todo!()
    }

    fn on_axis_event(&mut self, axis_name: StaticString, value: f32) -> bool {
        match axis_name {
            "player_move_right_left" => {
                self.linear_input.x = value;
                true
            }
            "player_move_up_down" => {
                self.linear_input.y = value;
                true
            }
            "player_move_forward_back" => {
                self.linear_input.z = value;
                true
            }
            "player_move_yaw" => {
                self.angular_input.y = value;
                true
            }
            "player_move_pitch" => {
                self.angular_input.x = value;
                true
            }

            _ => false,
        }
    }

    fn on_text_event(&mut self, text: String) -> bool {
        todo!()
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
        self.scene_instance = world_data
            .scene
            .add_instance(self.transform.clone(), self.model.clone());
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
