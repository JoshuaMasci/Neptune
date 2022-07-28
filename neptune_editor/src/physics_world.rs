use crate::game::Transform;
use rapier3d_f64::na::{UnitQuaternion, Vector3};
use rapier3d_f64::prelude::*;

pub enum Collider {
    Box(glam::DVec3),
    Sphere(f64),
    CapsuleY(f64, f64),
}

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
        let mut collider_set = ColliderSet::new();

        let integration_parameters = IntegrationParameters::default();
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = BroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let impulse_joint_set = ImpulseJointSet::new();
        let multibody_joint_set = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();

        let ground_collider = ColliderBuilder::cuboid(128.0, 1.0, 128.0)
            .translation(Vector::new(0.0, -10.0, 0.0))
            .build();
        collider_set.insert(ground_collider);

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

    // pub fn update_entity_transform(
    //     &self,
    //     rigid_body_handle: rapier3d_f64::prelude::RigidBodyHandle,
    //     transform: &mut Transform,
    // ) {
    //     if let Some(rigid_body) = self.rigid_body_set.get(rigid_body_handle) {
    //         transform.position = glam::DVec3::from_slice(rigid_body.translation().as_slice());
    //         let rotation = rigid_body.rotation();
    //         transform.rotation = glam::DQuat {
    //             x: rotation.i,
    //             y: rotation.j,
    //             z: rotation.k,
    //             w: rotation.w,
    //         }
    //         .as_f32();
    //     }
    // }

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

    pub fn add_collider(
        &mut self,
        rigid_body_handle: rapier3d_f64::prelude::RigidBodyHandle,
        transform: &Transform,
        collider: &Collider,
    ) -> rapier3d_f64::prelude::ColliderHandle {
        let collider = match collider {
            Collider::Box(half_extent) => {
                ColliderBuilder::cuboid(half_extent.x, half_extent.y, half_extent.z)
            }
            Collider::Sphere(radius) => ColliderBuilder::ball(*radius),
            Collider::CapsuleY(radius, half_height) => {
                ColliderBuilder::capsule_y(*half_height, *radius)
            }
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

        self.collider_set
            .insert_with_parent(collider, rigid_body_handle, &mut self.rigid_body_set)
    }

    pub(crate) fn remove_collider(
        &mut self,
        collider_handle: rapier3d_f64::prelude::ColliderHandle,
    ) {
        let _ = self.collider_set.remove(
            collider_handle,
            &mut self.island_manager,
            &mut self.rigid_body_set,
            true,
        );
    }

    pub(crate) fn get_mut_rigid_body(
        &mut self,
        rigid_body_handle: Option<rapier3d_f64::prelude::RigidBodyHandle>,
    ) -> Option<RigidBodyRef> {
        if let Some(rigid_body_handle) = rigid_body_handle {
            self.rigid_body_set
                .get_mut(rigid_body_handle)
                .map(|rigid_body| RigidBodyRef { rigid_body })
        } else {
            None
        }
    }
}

pub struct RigidBodyRef<'a> {
    rigid_body: &'a mut rapier3d_f64::prelude::RigidBody,
}

impl<'a> RigidBodyRef<'a> {
    pub fn get_transform(&self, transform: &mut Transform) {
        let position = self.rigid_body.translation().data.0[0];
        let rotation = self.rigid_body.rotation();

        transform.position = glam::DVec3::from_array(position);
        transform.rotation =
            glam::DQuat::from_array([rotation.w, rotation.i, rotation.j, rotation.k]);
    }

    pub fn get_linear_velocity(&self) -> glam::DVec3 {
        let velocity = self.rigid_body.linvel();
        glam::DVec3::from_array(velocity.data.0[0])
    }

    pub fn set_linear_velocity(&mut self, linear_velocity: glam::DVec3) {
        self.rigid_body.set_linvel(
            rapier3d_f64::prelude::Vector::from_column_slice(&linear_velocity.to_array()),
            true,
        );
    }
}
