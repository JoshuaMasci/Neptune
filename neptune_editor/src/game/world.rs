use crate::game::player::Player;
use crate::game::{Object, PlayerInput};
use crate::physics_world::PhysicsWorld;
use rapier3d_f64::prelude::ColliderHandle;

pub struct World {
    pub physics: PhysicsWorld,

    pub player: Option<Player>,
    pub objects: Vec<(Object, Option<ColliderHandle>)>,
}

impl World {
    pub fn new() -> Self {
        Self {
            physics: PhysicsWorld::new(),
            player: None,
            objects: Vec::new(),
        }
    }

    pub fn update(&mut self, delta_time: f32, input: &PlayerInput) {
        if let Some(player) = &mut self.player {
            player.update_input(input);
            player.update(delta_time, &mut self.physics);
        }
    }

    pub fn add_player(&mut self, mut player: Player) -> Option<Player> {
        let old_player = self.remove_player();

        player.add_to_world(&mut self.physics);

        old_player
    }

    pub fn remove_player(&mut self) -> Option<Player> {
        let mut old_player = self.player.take();

        if let Some(player) = &mut old_player {
            player.remove_from_world(&mut self.physics)
        }

        old_player
    }

    pub fn add_object(&mut self, object: Object) {
        self.objects.push((object, None));
    }
}
