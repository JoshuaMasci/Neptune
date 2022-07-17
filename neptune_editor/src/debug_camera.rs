use crate::camera::Camera;
use crate::transform::Transform;

pub struct DebugCamera {
    pub camera: Camera,
    pub transform: Transform,

    linear_speed: f32,

    x_input: [bool; 2],
    y_input: [bool; 2],
    z_input: [bool; 2],
}

impl DebugCamera {
    pub fn new() -> Self {
        Self {
            camera: Default::default(),
            transform: Default::default(),
            linear_speed: 5.0,
            x_input: [false, false],
            y_input: [false, false],
            z_input: [false, false],
        }
    }

    pub fn keyboard_input(
        &mut self,
        key: winit::event::VirtualKeyCode,
        state: winit::event::ElementState,
    ) {
        let pressed = state == winit::event::ElementState::Pressed;
        match key {
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

    pub fn update(&mut self, delta_time: f32) {
        let mut linear_movement = glam::Vec3::default();

        if self.x_input[0] {
            linear_movement.x += 1.0;
        }
        if self.x_input[1] {
            linear_movement.x -= 1.0;
        }

        if self.y_input[0] {
            linear_movement.y += 1.0;
        }
        if self.y_input[1] {
            linear_movement.y -= 1.0;
        }

        if self.z_input[0] {
            linear_movement.z += 1.0;
        }
        if self.z_input[1] {
            linear_movement.z -= 1.0;
        }

        linear_movement *= self.linear_speed * delta_time;

        self.transform.position += self.transform.get_right() * linear_movement.x as f64;
        self.transform.position += self.transform.get_up() * linear_movement.y as f64;
        self.transform.position += self.transform.get_forward() * linear_movement.z as f64;
    }
}
