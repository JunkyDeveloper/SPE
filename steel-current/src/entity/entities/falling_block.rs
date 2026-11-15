//! Steel: `objects/items/falling_block.rs`. Non-living: overrides `tick`.
//! Drops `fall_speed` per tick and "places" its block (logs + discards) at y = 0.

use std::sync::Arc;

use crate::entity::*;
use crate::util::Vec3;

pub struct FallingBlockEntity {
    base: EntityBase,
    block: &'static str,
    fall_speed: f64,
}

impl FallingBlockEntity {
    pub fn new(id: i32, position: Vec3, block: &'static str, fall_speed: f64) -> Self {
        Self { base: EntityBase::new(id, "falling_block", position), block, fall_speed }
    }
}

impl Entity for FallingBlockEntity {
    fn base(&self) -> &EntityBase {
        &self.base
    }

    fn tick(self: Arc<Self>) {
        if self.block == "air" {
            self.discard();
            return;
        }
        self.default_tick();
        let mut pos = self.position();
        pos.y = (pos.y - self.fall_speed).max(0.0);
        self.set_position(pos);
        if pos.y == 0.0 {
            self.log(&format!("placed {}", self.block));
            self.discard();
        }
    }
}
