use glam::Mat4;

#[derive(Debug, Clone, Copy)]
pub enum FieldOfView {
    X(f32),
    Y(f32),
}

impl FieldOfView {
    fn get_fov_y_rad(&self, aspect_ratio: f32) -> f32 {
        match self {
            FieldOfView::X(fov_x) => {
                f32::atan(f32::tan(fov_x.to_radians() / 2.0) / aspect_ratio) * 2.0
            }
            FieldOfView::Y(fov_y) => fov_y.to_radians(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub fov: FieldOfView,
    pub near_clip: f32,
    pub far_clip: Option<f32>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: FieldOfView::X(75.0),
            near_clip: 0.1,
            far_clip: Some(1000.0),
        }
    }
}

impl Camera {
    pub fn new(fov: FieldOfView, near_clip: f32, far_clip: Option<f32>) -> Self {
        Self {
            fov,
            near_clip,
            far_clip,
        }
    }

    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        let fov_y = self.fov.get_fov_y_rad(aspect_ratio);

        let mut matrix = if let Some(far_clip) = self.far_clip {
            Mat4::perspective_lh(fov_y, aspect_ratio, self.near_clip, far_clip)
        } else {
            Mat4::perspective_infinite_lh(fov_y, aspect_ratio, self.near_clip)
        };
        matrix.y_axis.y *= -1.0;
        matrix
    }
}
