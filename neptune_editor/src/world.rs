use na::Matrix4;

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
    pub fn get_perspective_matrix(&self, size: [u32; 2]) -> Matrix4<f32> {
        let aspect_ratio = size[0] as f32 / size[1] as f32;
        *na::Perspective3::new(
            aspect_ratio,
            f32::atan(f32::tan(self.fov_x_deg.to_radians() / 2.0) / aspect_ratio) * 2.0,
            self.z_near,
            self.z_far,
        )
        .as_matrix()
    }
}

#[derive(Default)]
pub struct Transform {
    pub position: na::Vector3<f64>,
    pub rotation: na::UnitQuaternion<f32>,
    pub scale: na::Vector3<f32>,
}

impl Transform {
    pub fn get_offset_model_matrix(&self, position_offset: na::Vector3<f64>) -> na::Matrix4<f32> {
        let position: na::Vector3<f64> = self.position - position_offset;

        let translation: na::Matrix4<f32> = na::Matrix4::new_translation(&na::Vector3::new(
            position.x as f32,
            position.y as f32,
            position.z as f32,
        ));

        let rotation: na::Matrix4<f32> = self.rotation.to_homogeneous();
        let scale: na::Matrix4<f32> = na::Matrix4::new_nonuniform_scaling(&self.scale);
        scale * rotation * translation
    }

    pub fn get_centered_view_matrix(&self) -> na::Matrix4<f32> {
        let eye = na::Point3::from(na::Vector3::zeros());
        let target = na::Point3::from(self.rotation * -na::Vector3::z_axis().into_inner());
        let up = self.rotation * na::Vector3::y_axis();
        na::Matrix4::look_at_lh(&eye, &target, &up)
    }
}

#[derive(Default)]
pub struct World {
    pub camera: Camera,
    pub camera_transform: Transform,
    pub entities: Vec<Transform>,
}
