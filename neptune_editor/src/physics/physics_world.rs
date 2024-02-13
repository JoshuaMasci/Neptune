use crate::transform::Transform;
use rapier3d::na::{UnitQuaternion, Vector3};
use rapier3d::prelude::*;

#[derive(Clone)]
pub enum Collider {
    Box(glam::Vec3),
    Sphere(f32),
    CapsuleY(f32, f32),
}

pub struct PhysicsWorld {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub query_pipeline: QueryPipeline,

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
        let query_pipeline = QueryPipeline::new();

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
            query_pipeline,
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
        let gravity = vector![0.0, -9.8, 0.0];

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
            None,
            &physics_hooks,
            &event_handler,
        );
    }

    pub fn add_rigid_body(&mut self, transform: &Transform) -> RigidBodyHandle {
        self.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .translation(Vector3::from_column_slice(&transform.position.to_array()))
                .rotation(
                    UnitQuaternion::from_quaternion(
                        rapier3d::na::Quaternion::new(
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

    pub fn remove_rigid_body(&mut self, rigid_body_handle: RigidBodyHandle) {
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
        rigid_body_handle: Option<RigidBodyHandle>,
        transform: &Transform,
        collider: &Collider,
    ) -> ColliderHandle {
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
                rapier3d::na::Quaternion::new(
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

        if let Some(rigid_body_handle) = rigid_body_handle {
            self.collider_set.insert_with_parent(
                collider,
                rigid_body_handle,
                &mut self.rigid_body_set,
            )
        } else {
            self.collider_set.insert(collider)
        }
    }

    pub(crate) fn remove_collider(&mut self, collider_handle: ColliderHandle) {
        let _ = self.collider_set.remove(
            collider_handle,
            &mut self.island_manager,
            &mut self.rigid_body_set,
            true,
        );
    }

    pub(crate) fn update_collider_transform(
        &mut self,
        collider_handle: ColliderHandle,
        transform: &Transform,
    ) {
        if let Some(collider) = self.collider_set.get_mut(collider_handle) {
            collider.set_translation(Vector3::from_column_slice(&transform.position.to_array()));
            collider.set_rotation(UnitQuaternion::from_quaternion(
                rapier3d::na::Quaternion::new(
                    transform.rotation.w,
                    transform.rotation.x,
                    transform.rotation.y,
                    transform.rotation.z,
                )
                .cast(),
            ));
        }
    }

    pub(crate) fn get_mut_rigid_body(
        &mut self,
        rigid_body_handle: Option<RigidBodyHandle>,
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
    rigid_body: &'a mut RigidBody,
}

impl<'a> RigidBodyRef<'a> {
    pub fn temp_player_settings(&mut self) {
        self.rigid_body
            .set_enabled_rotations(false, false, false, true);
        self.rigid_body.set_gravity_scale(0.0, true);
    }

    pub fn get_transform(&self, transform: &mut Transform) {
        let position = self.rigid_body.translation().data.0[0];
        let rotation = self.rigid_body.rotation();

        transform.position = glam::Vec3::from_array(position);
        transform.rotation =
            glam::Quat::from_array([rotation.i, rotation.j, rotation.k, rotation.w]);
    }

    pub fn get_linear_velocity(&self) -> glam::Vec3 {
        let velocity = self.rigid_body.linvel();
        glam::Vec3::from_array(velocity.data.0[0])
    }

    pub fn set_linear_velocity(&mut self, linear_velocity: glam::Vec3) {
        self.rigid_body
            .set_linvel(Vector::from_column_slice(&linear_velocity.to_array()), true);
    }

    pub fn set_angular_velocity(&mut self, angular_velocity: glam::Vec3) {
        self.rigid_body.set_angvel(
            Vector::from_column_slice(&angular_velocity.to_array()),
            true,
        );
    }
}
