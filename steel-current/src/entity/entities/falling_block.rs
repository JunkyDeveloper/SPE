//! Steel: `objects/items/falling_block.rs`. Non-living.
//! Position is a closed-form linear function of the tick count:
//! `y(t) = max(0, start_y - fall_speed * t)`.

use crate::entity::*;
use crate::util::Vec3;

pub struct FallingBlockEntity {
    base: EntityBase,
    start_y: f64,
    fall_speed: f64,
}

impl FallingBlockEntity {
    pub fn new(id: u32, x: f64, start_y: f64, z: f64, fall_speed: f64) -> Self {
        Self { base: EntityBase::new(id, "falling_block", Vec3::new(x, start_y, z)), start_y, fall_speed }
    }
}

impl Entity for FallingBlockEntity {
    fn base(&self) -> &EntityBase {
        &self.base
    }

    fn tick(&self) {
        self.default_tick();
        let t = f64::from(self.tick_count());
        let y = (self.start_y - self.fall_speed * t).max(0.0);
        let mut pos = self.position();
        pos.y = y;
        self.set_position(pos);
        if y == 0.0 {
            self.log("landed");
            self.set_removed();
        }
    }
}
