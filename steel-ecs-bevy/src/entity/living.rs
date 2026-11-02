//! Steel: `living_entity.rs` / `living_base`. Data: health. Tick step: regenerate.

use bevy_ecs::prelude::{Component, Query, ResMut, Without};

use crate::entity::{EntityId, Name, Removed, TickCount};
use crate::log::Logs;

#[derive(Component)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    pub fn new(current: i32, max: i32) -> Self {
        Self { current, max }
    }
}

/// +1 health every 20 ticks until full.
pub fn tick_regeneration(
    mut logs: ResMut<Logs>,
    mut living: Query<(&EntityId, &Name, &TickCount, &mut Health), Without<Removed>>,
) {
    for (id, name, tick, mut health) in &mut living {
        if !tick.0.is_multiple_of(20) || health.current >= health.max {
            continue;
        }
        health.current += 1;
        let msg = format!("health {}/{}", health.current, health.max);
        logs.entity(*id, name.0, tick.0, &msg);
    }
}
