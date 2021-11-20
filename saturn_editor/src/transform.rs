use nalgebra;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformF {
    position: nalgebra::Vector3<f32>,
    rotation: nalgebra::UnitQuaternion<f32>,
    scale: nalgebra::Vector3<f32>,
}

impl TransformF {
    pub fn new() -> Self {
        Self {
            position: nalgebra::zero(),
            rotation: Default::default(),
            scale: nalgebra::Vector3::new(0.0, 0.0, 0.0),
        }
    }
}
