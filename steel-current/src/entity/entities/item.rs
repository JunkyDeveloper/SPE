//! Steel: `objects/items/item_entity.rs`. Non-living: overrides `tick`, no living chain.
//! Pickup delay, despawn after `LIFETIME`, destroyed by damage (e.g. fire).

use std::sync::Arc;

use crate::entity::*;
use crate::util::{ItemStack, SyncMutex, Vec3};

pub const LIFETIME: i32 = 200; // vanilla 6000

pub struct ItemState {
    pub item: ItemStack,
    pub age: i32,
    pub pickup_delay: i32,
    pub health: f32,
}

pub struct ItemEntity {
    base: EntityBase,
    item_state: SyncMutex<ItemState>,
}

impl ItemEntity {
    pub fn new(id: i32, position: Vec3, item: ItemStack) -> Self {
        Self {
            base: EntityBase::new(id, "item", position),
            item_state: SyncMutex::new(ItemState { item, age: 0, pickup_delay: 10, health: 5.0 }),
        }
    }
    pub fn get_item(&self) -> ItemStack {
        self.item_state.lock().item
    }
    pub fn set_item(&self, item: ItemStack) {
        self.item_state.lock().item = item;
    }
    pub fn age(&self) -> i32 {
        self.item_state.lock().age
    }
}

impl Entity for ItemEntity {
    fn base(&self) -> &EntityBase {
        &self.base
    }
    fn as_item_entity(&self) -> Option<&ItemEntity> {
        Some(self)
    }

    fn tick(self: Arc<Self>) {
        if self.get_item().is_empty() {
            self.discard();
            return;
        }
        self.default_tick();
        let age = {
            let mut s = self.item_state.lock();
            if s.pickup_delay > 0 {
                s.pickup_delay -= 1;
            }
            s.age += 1;
            s.age
        };
        if age >= LIFETIME && !self.is_removed() {
            self.log("despawned");
            self.discard();
        }
    }

    fn hurt(&self, source: DamageSource, amount: f32) -> bool {
        let destroyed = {
            let mut s = self.item_state.lock();
            s.health -= amount;
            s.health <= 0.0
        };
        if destroyed && !self.is_removed() {
            self.log(&format!("destroyed by {}", source.kind));
            self.discard();
        }
        true
    }
}
