///A large world transform class
#[derive(Clone, Debug)]
pub struct Transform {
    pub position: glam::DVec3,
    pub rotation: glam::DQuat,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: glam::DVec3::ZERO,
            rotation: glam::DQuat::IDENTITY,
        }
    }
}

impl Transform {
    pub fn get_offset_model_matrix(&self, position_offset: glam::DVec3) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::ONE,
            self.rotation.as_f32(),
            (self.position - position_offset).as_vec3(),
        )
    }

    pub fn get_view_matrix(&self) -> glam::Mat4 {
        let position = self.position.as_vec3();
        let rotation = self.rotation.as_f32();
        glam::Mat4::look_at_lh(
            position,
            (rotation * glam::Vec3::Z) + position,
            rotation * glam::Vec3::Y,
        )
    }

    pub fn get_centered_view_matrix(&self) -> glam::Mat4 {
        let rotation = self.rotation.as_f32();

        glam::Mat4::look_at_lh(
            glam::Vec3::default(),
            rotation * glam::Vec3::Z,
            rotation * glam::Vec3::Y,
        )
    }

    pub fn get_right(&self) -> glam::DVec3 {
        self.rotation * glam::DVec3::X
    }

    pub fn get_up(&self) -> glam::DVec3 {
        self.rotation * glam::DVec3::Y
    }

    pub fn get_forward(&self) -> glam::DVec3 {
        self.rotation * glam::DVec3::Z
    }
}
