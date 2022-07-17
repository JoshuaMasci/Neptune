///A large world transform class, the position is stored in f64, while the rotation and scale are still in f32
#[derive(Default)]
pub struct Transform {
    pub position: glam::DVec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

impl Transform {
    pub fn get_offset_model_matrix(&self, position_offset: glam::DVec3) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(
            self.scale,
            self.rotation,
            (self.position - position_offset).as_vec3(),
        )
    }

    pub fn get_view_matrix(&self) -> glam::Mat4 {
        let position = self.position.as_vec3();
        glam::Mat4::look_at_lh(
            position,
            (self.rotation * glam::Vec3::Z) + position,
            self.rotation * glam::Vec3::Y,
        )
    }

    pub fn get_centered_view_matrix(&self) -> glam::Mat4 {
        glam::Mat4::look_at_lh(
            glam::Vec3::default(),
            self.rotation * glam::Vec3::Z,
            self.rotation * glam::Vec3::Y,
        )
    }

    pub fn get_right(&self) -> glam::DVec3 {
        (self.rotation * glam::Vec3::X).as_dvec3()
    }

    pub fn get_up(&self) -> glam::DVec3 {
        (self.rotation * glam::Vec3::Y).as_dvec3()
    }

    pub fn get_forward(&self) -> glam::DVec3 {
        (self.rotation * glam::Vec3::Z).as_dvec3()
    }
}
