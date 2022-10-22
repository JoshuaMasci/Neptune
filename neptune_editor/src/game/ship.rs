use crate::game::{Model, Transform};
use crate::rendering::mesh::Mesh;
use rapier3d_f64::prelude::{ColliderHandle, RigidBodyHandle};
use std::sync::Arc;

#[allow(dead_code)]
pub struct Ship {
    pub(crate) transform: Transform,

    rigid_body: Option<RigidBodyHandle>,
    collider: Option<ColliderHandle>,

    pub model: Option<Model>,
}
