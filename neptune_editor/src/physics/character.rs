use crate::physics::physics_world::PhysicsWorld;
use crate::physics::vec3_glam_to_na;
use glam::Vec3;
use rapier3d::control::CharacterLength;
use rapier3d::geometry::ShapeType::Capsule;
use rapier3d::na::{
    Isometry3, Quaternion, Translation, Translation3, UnitQuaternion, UnitVector3, Vector3,
};
use rapier3d::pipeline::QueryFilter;

// Goals of this struct is to abstract character movement behaviour
// Zero-G will probably require a separate controller
pub struct Character {
    controller: rapier3d::control::KinematicCharacterController,
}

impl Character {
    pub fn new(world: &mut PhysicsWorld) -> Self {
        let mut controller = rapier3d::control::KinematicCharacterController::default();
        controller.offset = CharacterLength::Relative(0.01);

        controller.max_slope_climb_angle = 45.0f32.to_radians();
        controller.min_slope_slide_angle = 30.0f32.to_radians();

        controller.autostep = Some(rapier3d::control::CharacterAutostep {
            max_height: CharacterLength::Relative(0.2),
            min_width: CharacterLength::Relative(0.2),
            include_dynamic_bodies: true,
        });

        Self { controller }
    }

    pub fn update(
        &mut self,
        world: &mut PhysicsWorld,
        character_velocity: &mut Vec3,
        gravity_vector: Vec3,
        delta_time: f32,
    ) {
        self.controller.up = UnitVector3::new_normalize(vec3_glam_to_na(&-gravity_vector));

        let shape = rapier3d::geometry::ColliderBuilder::capsule_y(1.8, 0.3).build();
        let position = Translation3::from(Vector3::default());
        let rotation = UnitQuaternion::new_normalize(Quaternion::default());

        let transform = Isometry3::from_parts(position, rotation);

        let movement = self.controller.move_shape(
            delta_time,
            &world.rigid_body_set,
            &world.collider_set,
            &world.query_pipeline,
            shape.shape(),
            &transform,
            vec3_glam_to_na(character_velocity),
            QueryFilter::default(),
            |_| {},
        );
    }
}
