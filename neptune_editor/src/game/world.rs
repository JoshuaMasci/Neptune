use crate::game::physics_world::PhysicsWorld;
use crate::game::player::Player;
use crate::game::{Object, PlayerInput, Transform};
use crate::rendering::camera::Camera;
use rapier3d_f64::prelude::{ColliderHandle, RigidBodyHandle};

pub struct DynamicObjectInstance {
    pub(crate) object: Object,
    rigid_body_handle: Option<RigidBodyHandle>,
    collider: Option<ColliderHandle>,
}

pub struct World {
    pub physics: PhysicsWorld,

    pub player: Option<Player>,

    pub static_objects: Vec<(Object, Option<ColliderHandle>)>,
    pub dynamic_objects: Vec<DynamicObjectInstance>,
}

impl World {
    pub fn new() -> Self {
        Self {
            physics: PhysicsWorld::new(),
            player: None,
            static_objects: Vec::new(),
            dynamic_objects: Vec::new(),
        }
    }

    pub fn get_camera_info(&self) -> (Camera, Transform) {
        let camera = self
            .player
            .as_ref()
            .map(|player| player.camera.clone())
            .unwrap_or_default();
        let transform = self
            .player
            .as_ref()
            .map(|player| player.transform.clone())
            .unwrap_or_default();
        (camera, transform)
    }

    pub fn update(&mut self, delta_time: f32, input: &PlayerInput) {
        self.physics.step(delta_time);

        for dynamic_object in self.dynamic_objects.iter_mut() {
            if let Some(rigid_body) = self
                .physics
                .get_mut_rigid_body(dynamic_object.rigid_body_handle)
            {
                rigid_body.get_transform(&mut dynamic_object.object.transform);
            }
        }

        if let Some(player) = &mut self.player {
            player.update_input(input);
            player.update(delta_time, &mut self.physics);
        }
    }

    pub fn add_player(&mut self, mut player: Player) -> Option<Player> {
        let old_player = self.remove_player();

        player.add_to_world(&mut self.physics);
        self.player = Some(player);

        old_player
    }

    pub fn remove_player(&mut self) -> Option<Player> {
        let mut old_player = self.player.take();

        if let Some(player) = &mut old_player {
            player.remove_from_world(&mut self.physics)
        }

        old_player
    }

    pub fn add_static_object(&mut self, object: Object) {
        let collider = object
            .collider
            .as_ref()
            .map(|collider| self.physics.add_collider(None, &object.transform, collider));
        self.static_objects.push((object, collider));
    }

    pub fn add_dynamic_object(&mut self, object: Object) {
        let rigid_body_handle = Some(self.physics.add_rigid_body(&object.transform));
        let collider = object.collider.as_ref().map(|collider| {
            self.physics
                .add_collider(rigid_body_handle, &Transform::default(), collider)
        });
        self.dynamic_objects.push(DynamicObjectInstance {
            object,
            rigid_body_handle,
            collider,
        });
    }
}
