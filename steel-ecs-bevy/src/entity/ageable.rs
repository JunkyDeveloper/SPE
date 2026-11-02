//! Steel: `ageable.rs`. Data: age. Tick step: age up until adult.

use bevy_ecs::prelude::{Component, Query, ResMut, Without};

use crate::entity::{EntityId, Name, Removed, TickCount};
use crate::log::Logs;

#[derive(Component)]
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

pub fn tick_ageable_mobs(
    mut logs: ResMut<Logs>,
    mut ageables: Query<(&EntityId, &Name, &TickCount, &mut Ageable), Without<Removed>>,
) {
    for (id, name, tick, mut ageable) in &mut ageables {
        if !ageable.is_baby() {
            continue;
        }
        ageable.age += 1;
        if !ageable.is_baby() {
            logs.entity(*id, name.0, tick.0, "grew up");
        }
    }
}
