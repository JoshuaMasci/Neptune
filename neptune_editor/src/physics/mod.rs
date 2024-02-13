mod character;
pub mod physics_world;

use glam::{Quat, Vec3};
use rapier3d::na::{Quaternion, Vector3};

pub fn vec3_glam_to_na(g_vec: &Vec3) -> Vector3<f32> {
    Vector3::new(g_vec.x, g_vec.y, g_vec.z)
}

pub fn vec3_na_to_glam(na_vec: &Vector3<f32>) -> Vec3 {
    Vec3::new(na_vec.x, na_vec.y, na_vec.z)
}

pub fn quat_glam_to_na(g_quat: &Quat) -> Quaternion<f32> {
    Quaternion::new(g_quat.w, g_quat.x, g_quat.y, g_quat.z)
}

pub fn quat_na_to_glam(na_quat: &Quaternion<f32>) -> Quat {
    Quat::from_xyzw(na_quat.w, na_quat.i, na_quat.j, na_quat.k)
}
