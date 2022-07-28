use crate::camera::Camera;
use crate::game::Transform;
use crate::physics_world::{Collider, PhysicsWorld};
use rapier3d_f64::prelude::{ColliderHandle, RigidBodyHandle};

#[derive(Default)]
pub struct PlayerInput {
    shoot_button: bool,
    interact_button: bool,
    x_input: [bool; 2],
    y_input: [bool; 2],
    z_input: [bool; 2],
}

impl PlayerInput {
    pub fn keyboard_input(
        &mut self,
        key: winit::event::VirtualKeyCode,
        state: winit::event::ElementState,
    ) {
        let pressed = state == winit::event::ElementState::Pressed;
        match key {
            winit::event::VirtualKeyCode::F => {
                self.interact_button = pressed;
            }
            winit::event::VirtualKeyCode::D => {
                self.x_input[0] = pressed;
            }
            winit::event::VirtualKeyCode::A => {
                self.x_input[1] = pressed;
            }
            winit::event::VirtualKeyCode::Space => {
                self.y_input[0] = pressed;
            }
            winit::event::VirtualKeyCode::LShift => {
                self.y_input[1] = pressed;
            }
            winit::event::VirtualKeyCode::W => {
                self.z_input[0] = pressed;
            }
            winit::event::VirtualKeyCode::S => {
                self.z_input[1] = pressed;
            }
            _ => {}
        }
    }
}

#[allow(dead_code)]
pub struct Player {
    transform: Transform,
    camera: Camera,
    camera_offset: Transform,

    rigid_body: Option<RigidBodyHandle>,
    collider: Option<ColliderHandle>,

    //TODO: need better input system
    shoot_input: bool,
    interact_input: bool,
    linear_input: glam::Vec3,
    angular_input: glam::Vec3,

    //Ground Movement
    ground_max_speed_x: f64,
    ground_max_speed_z: f64,
    ground_jump_power: f64,

    //Zero-G Movement
    zero_g_max_speed: glam::DVec3,
}

impl Player {
    pub fn new(transform: Transform, max_speed: f64) -> Self {
        Self {
            transform,
            camera: Default::default(),
            camera_offset: Default::default(),
            rigid_body: None,
            collider: None,
            shoot_input: false,
            interact_input: false,
            linear_input: glam::Vec3::ZERO,
            angular_input: glam::Vec3::ZERO,
            ground_max_speed_x: max_speed,
            ground_max_speed_z: max_speed,
            ground_jump_power: 1.0,
            zero_g_max_speed: glam::DVec3::splat(max_speed),
        }
    }

    pub fn add_to_world(&mut self, physics_world: &mut PhysicsWorld) {
        self.rigid_body = Some(physics_world.add_rigid_body(&self.transform));
        self.collider = Some(physics_world.add_collider(
            self.rigid_body.unwrap(),
            &Transform::default(),
            &Collider::CapsuleY(0.25, 0.9),
        ))
    }

    pub fn remove_from_world(&mut self, physics_world: &mut PhysicsWorld) {
        if let Some(collider) = self.collider {
            physics_world.remove_collider(collider);
        }

        if let Some(rigid_body) = self.rigid_body {
            physics_world.remove_rigid_body(rigid_body);
        }
    }

    pub fn update_input(&mut self, input: &PlayerInput) {
        self.linear_input.x += bool_to_float(input.x_input[0]);
        self.linear_input.x -= bool_to_float(input.x_input[1]);

        self.linear_input.y += bool_to_float(input.y_input[0]);
        self.linear_input.y -= bool_to_float(input.y_input[1]);

        self.linear_input.z += bool_to_float(input.z_input[0]);
        self.linear_input.z -= bool_to_float(input.z_input[1]);

        self.interact_input = input.interact_button;
        self.shoot_input = input.shoot_button;
    }

    pub fn update(&mut self, delta_time: f32, physics_world: &mut PhysicsWorld) {
        let in_gravity: bool = true;

        if let Some(mut rigid_body) = physics_world.get_mut_rigid_body(self.rigid_body) {
            rigid_body.get_transform(&mut self.transform);

            if in_gravity {
            } else {
                let some_value = 1.0;

                let linear_velocity = self.transform.rotation
                    * (self.zero_g_max_speed * self.linear_input.as_dvec3() * delta_time as f64);
                rigid_body.set_linear_velocity(linear_velocity);
            }

            //TODO: all the movement and stuff
        }
    }
}

fn bool_to_float(value: bool) -> f32 {
    if value {
        1.0
    } else {
        0.0
    }
}
