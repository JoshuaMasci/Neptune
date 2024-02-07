use crate::game::entity::{Entity, Player, StaticEntity};
use crate::scene::scene_renderer::Scene;

pub struct World {
    pub data: WorldData,
    pub entities: WorldEntities,
}

impl World {
    pub fn add_player(&mut self, mut player: Player) {
        player.add_to_world(&mut self.data);
        self.entities.player = Some(player);
    }

    pub fn add_static_entity(&mut self, mut static_entity: StaticEntity) {
        static_entity.add_to_world(&mut self.data);
        self.entities.static_entities.push(static_entity);
    }

    pub fn update(&mut self, delta_time: f32) {
        for entity in self.entities.static_entities.iter_mut() {
            entity.update(delta_time, &mut self.data);
        }

        if let Some(player) = &mut self.entities.player {
            player.update(delta_time, &mut self.data);
        }
    }
}

pub struct WorldData {
    pub scene: Scene,
    //TODO: Physics World
}

#[derive(Default)]
pub struct WorldEntities {
    pub(crate) player: Option<Player>,
    static_entities: Vec<StaticEntity>,
}
