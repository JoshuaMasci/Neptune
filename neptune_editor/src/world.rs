#[derive(Default)]
pub struct Transform {
    position: glm::DVec3,
    rotation: glm::Quat,
    scale: glm::Vec3,
}

impl Transform {
    pub fn get_offset_model_matrix(&self, position_offset: glm::DVec3) -> glm::Mat4 {
        let position = self.position - position_offset;
        let translation: glm::Mat4 = glm::translation(&glm::Vec3::new(
            position.x as f32,
            position.y as f32,
            position.z as f32,
        ));
        let rotation: glm::Mat4 = glm::quat_to_mat4(&self.rotation);
        let scale: glm::Mat4 = glm::scaling(&self.scale);
        scale * rotation * translation
    }

    // pub fn get_centered_view_matrix(&self) -> glam::Mat4 {
    //     let forward = self.rotation * glam::Vec3::new(0.0, 0.0, 1.0);
    //     let up = self.rotation * glam::Vec3::new(0.0, 1.0, 0.0);
    //     glam::Mat4::look_at_lh(glam::Vec3::splat(0.0), forward, up)
    // }
}

#[derive(Default)]
pub struct World {
    camera: Transform,
    entities: Vec<Transform>,
}
