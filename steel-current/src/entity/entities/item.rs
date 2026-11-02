//! Steel: `objects/items/item_entity.rs`. Non-living: overrides `tick`, no living chain.
//! Data: despawn countdown. Moves with constant velocity (via `base_tick`).

use crate::entity::*;
use crate::util::{SyncMutex, Vec3};

pub struct ItemEntity {
    base: EntityBase,
    despawn_in: SyncMutex<u32>,
}

impl ItemEntity {
    pub fn new(id: u32, position: Vec3, velocity: Vec3, lifetime: u32) -> Self {
        let base = EntityBase::new(id, "item", position);
        base.with(|s| s.velocity = velocity);
        Self { base, despawn_in: SyncMutex::new(lifetime) }
    }
}

impl Entity for ItemEntity {
    fn base(&self) -> &EntityBase {
        &self.base
    }

    fn tick(&self) {
        self.default_tick();
        let done = {
            let mut t = self.despawn_in.lock();
            *t = t.saturating_sub(1);
            *t == 0
        };
        if done {
            self.log(&format!("despawned at {:?}", self.position()));
            self.set_removed();
        }
    }
}
