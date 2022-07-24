use crate::transform::Transform;
use rapier3d_f64::na::{Quaternion, UnitQuaternion, Vector3};
use rapier3d_f64::prelude::*;

pub struct PhysicsWorld {
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,

    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        let rigid_body_set = RigidBodySet::new();
        let collider_set = ColliderSet::new();

        let integration_parameters = IntegrationParameters::default();
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = BroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let impulse_joint_set = ImpulseJointSet::new();
        let multibody_joint_set = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();

        Self {
            rigid_body_set,
            collider_set,
            integration_parameters,
            physics_pipeline,
            island_manager,
            broad_phase,
            narrow_phase,
            impulse_joint_set,
            multibody_joint_set,
            ccd_solver,
        }
    }

    pub fn step(&mut self, delta_time: f32) {
        let gravity = vector![0.0, -9.81, 0.0];

        let physics_hooks = ();
        let event_handler = ();

        self.integration_parameters.dt = delta_time as Real;

        self.physics_pipeline.step(
            &gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            &physics_hooks,
            &event_handler,
        );
    }

    pub fn add_rigid_body(
        &mut self,
        transform: &Transform,
    ) -> rapier3d_f64::prelude::RigidBodyHandle {
        self.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .translation(Vector3::from_column_slice(&transform.position.to_array()))
                .rotation(
                    UnitQuaternion::from_quaternion(
                        rapier3d_f64::na::Quaternion::new(
                            transform.rotation.w,
                            transform.rotation.x,
                            transform.rotation.y,
                            transform.rotation.z,
                        )
                        .cast(),
                    )
                    .scaled_axis(),
                )
                .build(),
        )
    }

    pub fn add_collider(
        &mut self,
        rigid_body_handle: rapier3d_f64::prelude::RigidBodyHandle,
        transform: &Transform,
        collider: &crate::entity::Collider,
    ) {
        let collider = match collider {
            crate::entity::Collider::Box(half_extent) => {
                ColliderBuilder::cuboid(half_extent.x, half_extent.y, half_extent.z)
            }
            crate::entity::Collider::Sphere(radius) => ColliderBuilder::ball(*radius),
        }
        .translation(Vector3::from_column_slice(&transform.position.to_array()))
        .rotation(
            UnitQuaternion::from_quaternion(
                rapier3d_f64::na::Quaternion::new(
                    transform.rotation.w,
                    transform.rotation.x,
                    transform.rotation.y,
                    transform.rotation.z,
                )
                .cast(),
            )
            .scaled_axis(),
        )
        .build();

        let _handle = self.collider_set.insert_with_parent(
            collider,
            rigid_body_handle,
            &mut self.rigid_body_set,
        );
    }

    pub fn update_rigid_body_transform(
        &mut self,
        rigid_body_handle: rapier3d_f64::prelude::RigidBodyHandle,
        transform: &Transform,
    ) {
        if let Some(rigid_body) = self.rigid_body_set.get_mut(rigid_body_handle) {
            rigid_body.set_translation(
                Vector3::from_column_slice(&transform.position.to_array()),
                true,
            );
            rigid_body.set_rotation(
                UnitQuaternion::from_quaternion(
                    rapier3d_f64::na::Quaternion::new(
                        transform.rotation.w,
                        transform.rotation.x,
                        transform.rotation.y,
                        transform.rotation.z,
                    )
                    .cast(),
                )
                .scaled_axis(),
                true,
            );
        }
    }

    pub fn update_entity_transform(
        &self,
        rigid_body_handle: rapier3d_f64::prelude::RigidBodyHandle,
        transform: &mut Transform,
    ) {
        if let Some(rigid_body) = self.rigid_body_set.get(rigid_body_handle) {
            transform.position = glam::DVec3::from_slice(rigid_body.translation().as_slice());
            let rotation = rigid_body.rotation();
            transform.rotation = glam::DQuat {
                x: rotation.i,
                y: rotation.j,
                z: rotation.k,
                w: rotation.w,
            }
            .as_f32();
        }
    }

    pub fn remove_rigid_body(&mut self, rigid_body_handle: rapier3d_f64::prelude::RigidBodyHandle) {
        let _ = self.rigid_body_set.remove(
            rigid_body_handle,
            &mut self.island_manager,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            true,
        );
    }
}
