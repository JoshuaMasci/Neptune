use crate::physics::physics_world::PhysicsWorld;
use crate::physics::{quat_glam_to_na, vec3_glam_to_na};
use crate::transform::Transform;
use glam::Vec3;
use rapier3d::control::CharacterLength;
use rapier3d::na::{Isometry3, Translation3, UnitQuaternion, UnitVector3, Vector3};
use rapier3d::pipeline::QueryFilter;

// Goals of this struct is to abstract character movement behaviour
// Zero-G will probably require a separate controller
pub struct CharacterController {
    controller: rapier3d::control::KinematicCharacterController,
    collision_handle: Option<rapier3d::geometry::ColliderHandle>,

    is_grounded: bool,
    is_sliding: bool,
}

impl CharacterController {
    pub fn new() -> Self {
        let controller = rapier3d::control::KinematicCharacterController {
            up: UnitVector3::new_normalize(Vector3::new(0.0, 1.0, 0.0)),
            offset: CharacterLength::Absolute(0.01),
            autostep: Some(rapier3d::control::CharacterAutostep {
                max_height: CharacterLength::Absolute(0.2),
                min_width: CharacterLength::Absolute(0.2),
                include_dynamic_bodies: true,
            }),
            max_slope_climb_angle: 45.0f32.to_radians(),
            min_slope_slide_angle: 30.0f32.to_radians(),
            snap_to_ground: Some(CharacterLength::Absolute(0.01)),
            ..Default::default()
        };

        Self {
            controller,
            collision_handle: None,
            is_grounded: false,
            is_sliding: false,
        }
    }

    pub fn add_to_world(&mut self, world: &mut PhysicsWorld, character_transform: &Transform) {
        let position = Translation3::from(vec3_glam_to_na(&character_transform.position));
        let rotation =
            UnitQuaternion::new_normalize(quat_glam_to_na(&character_transform.rotation));
        let transform = Isometry3::from_parts(position, rotation);
        let collider = rapier3d::geometry::ColliderBuilder::capsule_y(1.8, 0.3)
            .position(transform)
            .build();
        self.collision_handle = Some(world.collider_set.insert(collider));
    }

    pub fn remove_from_world(&mut self, world: &mut PhysicsWorld) {
        if let Some(handle) = self.collision_handle.take() {
            world.collider_set.remove(
                handle,
                &mut world.island_manager,
                &mut world.rigid_body_set,
                true,
            );
        }
    }

    pub fn update(
        &mut self,
        world: &mut PhysicsWorld,
        character_transform: &mut Transform,
        character_velocity: &Vec3,
        delta_time: f32,
    ) {
        if let Some(collider_handle) = &self.collision_handle {
            let position = Translation3::from(vec3_glam_to_na(&character_transform.position));
            let rotation =
                UnitQuaternion::new_normalize(quat_glam_to_na(&character_transform.rotation));
            let transform = Isometry3::from_parts(position, rotation);

            let collider_shape = world.collider_set.get(*collider_handle).unwrap().shape();

            let filter = QueryFilter::new().exclude_collider(*collider_handle);

            let mut collisions = vec![];
            let movement = self.controller.move_shape(
                delta_time,
                &world.rigid_body_set,
                &world.collider_set,
                &world.query_pipeline,
                collider_shape,
                &transform,
                vec3_glam_to_na(character_velocity),
                filter,
                |collision| collisions.push(collision),
            );

            for collision in collisions.iter() {
                self.controller.solve_character_collision_impulses(
                    delta_time,
                    &mut world.rigid_body_set,
                    &world.collider_set,
                    &world.query_pipeline,
                    collider_shape,
                    10.0,
                    collision,
                    filter,
                );
            }

            self.is_grounded = movement.grounded;
            self.is_sliding = movement.is_sliding_down_slope;
            character_transform.position += Vec3::from_array(*movement.translation.as_ref());
        }
    }

    pub fn on_ground(&self) -> bool {
        self.is_grounded
    }
}
