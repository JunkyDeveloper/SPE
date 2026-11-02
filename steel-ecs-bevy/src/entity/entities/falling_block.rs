//! Steel: `objects/items/falling_block.rs`. Non-living.
//! Position is a closed-form linear function of the tick count:
//! `y(t) = max(0, start_y - fall_speed * t)`.

use bevy_ecs::prelude::{Bundle, Commands, Component, Entity, Query, ResMut, Without};

use crate::entity::{EntityBaseBundle, EntityId, Name, Position, Removed, TickCount};
use crate::log::Logs;
use crate::util::Vec3;

#[derive(Component)]
pub struct FallingBlock {
    pub start_y: f64,
    pub fall_speed: f64,
}

#[derive(Bundle)]
pub struct FallingBlockBundle {
    pub base: EntityBaseBundle,
    pub block: FallingBlock,
}

pub fn bundle(id: EntityId, x: f64, start_y: f64, z: f64, fall_speed: f64) -> FallingBlockBundle {
    FallingBlockBundle {
        base: EntityBaseBundle::new(id, "falling_block", Vec3::new(x, start_y, z)),
        block: FallingBlock {
            start_y,
            fall_speed,
        },
    }
}

/// The part of `FallingBlockEntity::tick` after `default_tick`.
#[allow(clippy::type_complexity)] // clippy dislikes any non-trivial bevy Query type
pub fn tick_falling_blocks(
    mut commands: Commands,
    mut logs: ResMut<Logs>,
    mut blocks: Query<
        (
            Entity,
            &EntityId,
            &Name,
            &TickCount,
            &FallingBlock,
            &mut Position,
        ),
        Without<Removed>,
    >,
) {
    for (entity, id, name, tick, block, mut position) in &mut blocks {
        let t = f64::from(tick.0);
        let y = (block.start_y - block.fall_speed * t).max(0.0);
        position.0.y = y;
        if y == 0.0 {
            logs.entity(*id, name.0, tick.0, "landed");
            commands.entity(entity).insert(Removed);
        }
    }
}
