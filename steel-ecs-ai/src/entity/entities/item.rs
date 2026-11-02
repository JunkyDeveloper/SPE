//! Steel: `objects/items/item_entity.rs`. Non-living: no living chain, just the base tick and a
//! despawn countdown. Moves with constant velocity (via `apply_velocity`).

use crate::entity::{Cx, Entity, Kind, pipeline};
use crate::util::Vec3;

pub struct Item {
    pub despawn_in: u32,
}

pub static ITEM: Kind = Kind {
    name: "item",
    pipeline: pipeline::ITEM,
};

pub fn spawn(position: Vec3, velocity: Vec3, lifetime: u32) -> Entity {
    let mut entity = Entity::new(&ITEM, position);
    entity.base.velocity = velocity;
    entity.item = Some(Item {
        despawn_in: lifetime,
    });
    entity
}

pub fn despawn_countdown(e: &mut Entity, _cx: &mut Cx) {
    let item = e.item_mut();
    item.despawn_in = item.despawn_in.saturating_sub(1);
    if item.despawn_in == 0 {
        let position = e.base.position;
        e.log(format_args!("despawned at {position:?}"));
        e.base.removed = true;
    }
}
