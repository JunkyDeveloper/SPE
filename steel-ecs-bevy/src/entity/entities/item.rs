//! Steel: `objects/items/item_entity.rs`. Non-living: no living chain.
//! Data: despawn countdown. Moves with constant velocity (via the shared `base_tick` system).

use bevy_ecs::prelude::{Bundle, Commands, Component, Entity, Query, ResMut, Without};

use crate::entity::{EntityBaseBundle, EntityId, Name, Position, Removed, TickCount, Velocity};
use crate::log::Logs;
use crate::util::Vec3;

#[derive(Component)]
pub struct Item {
    pub despawn_in: u32,
}

#[derive(Bundle)]
pub struct ItemBundle {
    pub base: EntityBaseBundle,
    pub item: Item,
}

pub fn bundle(id: EntityId, position: Vec3, velocity: Vec3, lifetime: u32) -> ItemBundle {
    let mut base = EntityBaseBundle::new(id, "item", position);
    base.velocity = Velocity(velocity);
    ItemBundle {
        base,
        item: Item {
            despawn_in: lifetime,
        },
    }
}

/// The part of `ItemEntity::tick` after `default_tick`.
/// `Removed` is inserted through `Commands`: no collect-drop-guards-apply dance as in
/// steel-ecs-ai, the insert is queued and applied at the next sync point.
#[allow(clippy::type_complexity)] // clippy dislikes any non-trivial bevy Query type
pub fn tick_items(
    mut commands: Commands,
    mut logs: ResMut<Logs>,
    mut items: Query<
        (Entity, &EntityId, &Name, &TickCount, &Position, &mut Item),
        Without<Removed>,
    >,
) {
    for (entity, id, name, tick, position, mut item) in &mut items {
        item.despawn_in = item.despawn_in.saturating_sub(1);
        if item.despawn_in == 0 {
            logs.entity(
                *id,
                name.0,
                tick.0,
                &format!("despawned at {:?}", position.0),
            );
            commands.entity(entity).insert(Removed);
        }
    }
}
