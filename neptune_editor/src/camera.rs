//TODO: add orthographic/perspective camera modes
pub struct Camera {
    z_near: f32,
    z_far: f32,
    fov_x_deg: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            z_near: 0.1,
            z_far: 1000.0,
            fov_x_deg: 75.0,
        }
    }
}

impl Camera {
    pub fn get_perspective_matrix(&self, size: [u32; 2]) -> glam::Mat4 {
        let aspect_ratio = size[0] as f32 / size[1] as f32;
        let fov_y = f32::atan(f32::tan(self.fov_x_deg.to_radians() / 2.0) / aspect_ratio) * 2.0;
        glam::Mat4::perspective_lh(fov_y, aspect_ratio, self.z_near, self.z_far)
    }

    pub fn get_infinite_reverse_perspective_matrix(&self, size: [u32; 2]) -> glam::Mat4 {
        let aspect_ratio = size[0] as f32 / size[1] as f32;
        let fov_y = f32::atan(f32::tan(self.fov_x_deg.to_radians() / 2.0) / aspect_ratio) * 2.0;
        glam::Mat4::perspective_infinite_reverse_lh(fov_y, aspect_ratio, self.z_near)
    }
}
