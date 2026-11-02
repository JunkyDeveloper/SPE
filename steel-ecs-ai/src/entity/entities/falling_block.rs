//! Steel: `objects/items/falling_block.rs`. Non-living.
//! Position is a closed-form linear function of the tick count:
//! `y(t) = max(0, start_y - fall_speed * t)`.

use crate::ecs::{Entity, World};
use crate::entity::{EntityHeader, EntityId, Position, mark_removed, spawn_base};
use crate::log::Logs;
use crate::util::Vec3;

pub struct FallingBlock {
    pub start_y: f64,
    pub fall_speed: f64,
}

pub fn spawn(
    world: &mut World,
    id: EntityId,
    x: f64,
    start_y: f64,
    z: f64,
    fall_speed: f64,
) -> Entity {
    let entity = spawn_base(world, id, "falling_block", Vec3::new(x, start_y, z));
    world.insert(
        entity,
        FallingBlock {
            start_y,
            fall_speed,
        },
    );
    entity
}

/// The part of `FallingBlockEntity::tick` after `default_tick`.
pub fn tick_falling_blocks(world: &mut World) {
    let mut landed = Vec::new();
    {
        let header = EntityHeader::new(world);
        let mut positions = world.write::<Position>();
        let blocks = world.read::<FallingBlock>();
        let mut logs = world.res_mut::<Logs>();
        for (entity, block) in blocks.iter() {
            if !header.is_active(entity) {
                continue;
            }
            let t = f64::from(header.tick_count(entity));
            let y = (block.start_y - block.fall_speed * t).max(0.0);
            positions
                .get_mut(entity)
                .expect("falling block without Position")
                .0
                .y = y;
            if y == 0.0 {
                header.log(&mut logs, entity, "landed");
                landed.push(entity);
            }
        }
    } // guards dropped: `Removed` can be inserted now
    for entity in landed {
        mark_removed(world, entity);
    }
}
