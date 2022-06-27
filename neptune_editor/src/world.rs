#[derive(Default)]
pub struct Transform {
    position: glam::DVec3,
    rotation: glam::Quat,
    scale: glam::Vec3,
}

#[derive(Default)]
pub struct World {
    camera: Transform,
    entities: Vec<Transform>,
}
