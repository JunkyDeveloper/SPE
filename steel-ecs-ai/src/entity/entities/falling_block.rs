//! Steel: `objects/items/falling_block.rs`. Non-living.
//! Position is a closed-form linear function of the tick count:
//! `y(t) = max(0, start_y - fall_speed * t)`.

use crate::entity::{Cx, Entity, Kind, pipeline};
use crate::util::Vec3;

pub struct FallingBlock {
    pub start_y: f64,
    pub fall_speed: f64,
}

pub static FALLING_BLOCK: Kind = Kind {
    name: "falling_block",
    pipeline: pipeline::FALLING_BLOCK,
};

pub fn spawn(x: f64, start_y: f64, z: f64, fall_speed: f64) -> Entity {
    let mut entity = Entity::new(&FALLING_BLOCK, Vec3::new(x, start_y, z));
    entity.falling_block = Some(FallingBlock {
        start_y,
        fall_speed,
    });
    entity
}

pub fn fall(e: &mut Entity, _cx: &mut Cx) {
    let falling = e.falling_block();
    let t = f64::from(e.base.tick_count);
    let y = (falling.start_y - falling.fall_speed * t).max(0.0);
    e.base.position.y = y;
    if y == 0.0 {
        e.log("landed");
        e.base.removed = true;
    }
}
