use crate::renderer::Mesh;
use std::sync::Arc;

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

#[derive(Default)]
pub struct World {
    pub camera: Camera,
    pub camera_transform: Transform,
    pub entities: Vec<Entity>,
}

pub struct Entity {
    pub(crate) transform: Transform,
    pub(crate) mesh: Arc<Mesh>,
}
