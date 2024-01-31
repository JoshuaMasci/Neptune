use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn with_position(position: Vec3) -> Self {
        Self {
            position,
            ..Transform::default()
        }
    }

    pub fn with_rotation(rotation: Quat) -> Self {
        Self {
            rotation,
            ..Transform::default()
        }
    }

    pub fn with_scale(scale: Vec3) -> Self {
        Self {
            scale,
            ..Transform::default()
        }
    }

    pub fn translate(&mut self, translation: Vec3) {
        self.position += translation;
    }

    pub fn rotate(&mut self, axis: Vec3, angle: f32) {
        self.rotation = Quat::from_axis_angle(axis, angle) * self.rotation;
    }

    pub fn scale(&mut self, scale: Vec3) {
        self.scale *= scale;
    }

    pub fn model_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_lh(
            self.position,
            self.rotation * Vec3::Z,
            self.rotation * Vec3::Y,
        )
    }
}

impl From<Mat4> for Transform {
    fn from(value: Mat4) -> Self {
        let (scale, rotation, position) = value.to_scale_rotation_translation();
        Self {
            position,
            rotation,
            scale,
        }
    }
}
