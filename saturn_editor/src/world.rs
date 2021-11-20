use crate::transform::TransformF;
use legion;

pub struct World {
    world: legion::World,
    resources: legion::Resources,
    schedule: legion::Schedule,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pos(f32, f32, f32);

impl World {
    pub fn new() -> Self {
        let mut new = Self {
            world: legion::World::default(),
            resources: legion::Resources::default(),
            schedule: legion::Schedule::builder().build(),
        };

        let entity = new.world.push((TransformF::new(), 1u8));
        new.world.remove(entity);
        new
    }

    pub fn update(&mut self, time_step: f32) {
        self.schedule.execute(&mut self.world, &mut self.resources);
    }
}
