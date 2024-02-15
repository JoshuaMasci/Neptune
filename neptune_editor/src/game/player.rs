use crate::game::entity::Entity;
use crate::game::world::WorldData;
use crate::input::{ButtonState, InputEventReceiver, StaticString};
use crate::physics::character::CharacterController;
use crate::transform::Transform;
use glam::{Quat, Vec2, Vec3};

pub struct Player {
    transform: Transform,

    camera_pitch: f32,
    camera_offset: Vec3,

    character: CharacterController,
    gravity_velocity: Vec3,

    // Properties
    /// units: m/s
    linear_speed: Vec3,

    /// pitch yaw: rad/s
    angular_speed: Vec2,

    // Input
    linear_input: Vec3,
    angular_input: Vec2,
    is_sprinting: bool,
}

impl Player {
    pub fn with_position(position: Vec3) -> Self {
        Self {
            transform: Transform::with_position(position),

            camera_pitch: 0.0,
            camera_offset: Vec3::Y * 0.5,

            character: CharacterController::new(),
            gravity_velocity: Vec3::ZERO,

            linear_speed: Vec3::splat(5.0),
            angular_speed: Vec2::splat(std::f32::consts::PI),
            linear_input: Vec3::ZERO,
            angular_input: Vec2::ZERO,
            is_sprinting: false,
        }
    }

    pub fn get_camera_transform(&self) -> Transform {
        self.transform.transform(&Transform {
            position: self.camera_offset,
            rotation: Quat::from_axis_angle(Vec3::X, self.camera_pitch),
            scale: Vec3::ONE,
        })
    }
}

impl Entity for Player {
    fn add_to_world(&mut self, world_data: &mut WorldData) {
        self.character
            .add_to_world(&mut world_data.physics, &self.transform);
    }

    fn remove_from_world(&mut self, world_data: &mut WorldData) {
        self.character.remove_from_world(&mut world_data.physics);
    }

    fn update(&mut self, delta_time: f32, world_data: &mut WorldData) {
        let angular_movement = -self.angular_input * self.angular_speed * delta_time;

        // Clamp pitch 180 deg arc
        const PI_2: f32 = std::f32::consts::FRAC_PI_2;
        self.camera_pitch += angular_movement.x;
        self.camera_pitch = self.camera_pitch.clamp(-PI_2, PI_2);

        // Rotate player body
        let up = self.transform.rotation * Vec3::Y;
        self.transform.rotation *= Quat::from_axis_angle(up, angular_movement.y);

        let velocity = self.transform.rotation
            * (self.linear_input
                * self.linear_speed
                * delta_time
                * if self.is_sprinting { 10.0 } else { 1.0 });

        if !self.character.on_ground() {
            self.gravity_velocity += Vec3::NEG_Y * 9.8 * delta_time;
        } else {
            self.gravity_velocity = Vec3::ZERO;
        }

        self.character.update(
            &mut world_data.physics,
            &mut self.transform,
            &(velocity + self.gravity_velocity),
            delta_time,
        );
    }
}

impl InputEventReceiver for Player {
    fn requests_mouse_capture(&mut self) -> bool {
        todo!()
    }

    fn on_button_event(&mut self, button_name: StaticString, state: ButtonState) -> bool {
        match button_name {
            "player_move_sprint" => {
                self.is_sprinting = state.is_down();
                true
            }
            _ => false,
        }
    }

    fn on_axis_event(&mut self, axis_name: StaticString, value: f32) -> bool {
        match axis_name {
            "player_move_left_right" => {
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
                self.angular_input.x = -value;
                true
            }

            _ => false,
        }
    }

    fn on_text_event(&mut self, text: String) -> bool {
        todo!()
    }
}
