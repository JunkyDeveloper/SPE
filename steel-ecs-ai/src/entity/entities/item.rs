//! Steel: `objects/items/item_entity.rs`. Non-living: no living chain.
//! Data: despawn countdown. Moves with constant velocity (via the shared `base_tick` system).

use crate::ecs::{Entity, World};
use crate::entity::{EntityHeader, EntityId, Position, Velocity, mark_removed, spawn_base};
use crate::log::Logs;
use crate::util::Vec3;

pub struct Item {
    pub despawn_in: u32,
}

pub fn spawn(
    world: &mut World,
    id: EntityId,
    position: Vec3,
    velocity: Vec3,
    lifetime: u32,
) -> Entity {
    let entity = spawn_base(world, id, "item", position);
    world.insert(entity, Velocity(velocity));
    world.insert(
        entity,
        Item {
            despawn_in: lifetime,
        },
    );
    entity
}

/// The part of `ItemEntity::tick` after `default_tick`.
pub fn tick_items(world: &mut World) {
    let mut done = Vec::new();
    {
        let header = EntityHeader::new(world);
        let positions = world.read::<Position>();
        let mut items = world.write::<Item>();
        let mut logs = world.res_mut::<Logs>();
        for (entity, item) in items.iter_mut() {
            if !header.is_active(entity) {
                continue;
            }
            item.despawn_in = item.despawn_in.saturating_sub(1);
            if item.despawn_in == 0 {
                let position = positions.get(entity).expect("item without Position").0;
                header.log(&mut logs, entity, &format!("despawned at {position:?}"));
                done.push(entity);
            }
        }
    } // guards dropped: `Removed` can be inserted now
    for entity in done {
        mark_removed(world, entity);
    }
}
