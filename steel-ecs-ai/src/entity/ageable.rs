//! Steel: `ageable.rs`. Data: age. Tick step: age up until adult.

use crate::ecs::World;
use crate::entity::EntityHeader;
use crate::log::Logs;

pub struct Ageable {
    pub age: u32,
    /// Age at which the mob stops being a baby.
    pub adult_age: u32,
}

impl Ageable {
    pub fn new(age: u32, adult_age: u32) -> Self {
        Self { age, adult_age }
    }

    pub fn is_baby(&self) -> bool {
        self.age < self.adult_age
    }
}

pub fn tick_ageable_mobs(world: &mut World) {
    let header = EntityHeader::new(world);
    let mut ageables = world.write::<Ageable>();
    let mut logs = world.res_mut::<Logs>();
    for (entity, ageable) in ageables.iter_mut() {
        if !header.is_active(entity) || !ageable.is_baby() {
            continue;
        }
        ageable.age += 1;
        if !ageable.is_baby() {
            header.log(&mut logs, entity, "grew up");
        }
    }
}
