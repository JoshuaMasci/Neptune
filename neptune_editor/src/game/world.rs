use crate::game::player::Player;
use crate::game::PlayerInput;
use crate::physics_world::PhysicsWorld;

pub struct World {
    pub physics: PhysicsWorld,

    player: Option<Player>,
}

impl World {
    pub fn new() -> Self {
        Self {
            physics: PhysicsWorld::new(),
            player: None,
        }
    }

    pub fn update(&mut self, delta_time: f32, input: &PlayerInput) {
        if let Some(player) = &mut self.player {
            player.update_input(&input);
            player.update(delta_time, &mut self.physics);
        }
    }
}
