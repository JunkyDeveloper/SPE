//! Steel: `world/mod.rs` `tick_game`, `entity/manager`, `world/block_entity_ticker.rs`,
//! reduced to two lists. Kept from Steel: tick over a snapshot (things spawned mid-tick
//! first tick next tick), skip removed, drop removed ones after the pass.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::block_entity::{BlockEntity, SharedBlockEntity};
use crate::entity::{Entity, SharedEntity};
use crate::util::SyncMutex;

#[derive(Default)]
pub struct EntityManager {
    tick_list: SyncMutex<Vec<SharedEntity>>,
}

impl EntityManager {
    pub fn tick_entities(&self) {
        let snapshot = self.tick_list.lock().clone();
        for entity in snapshot {
            if entity.is_removed() {
                continue;
            }
            entity.advance_tick_count();
            entity.tick();
        }
        self.tick_list.lock().retain(|e| !e.is_removed());
    }
    pub fn entities(&self) -> Vec<SharedEntity> {
        self.tick_list.lock().clone()
    }
}

#[derive(Default)]
pub struct BlockEntityManager {
    tickers: SyncMutex<Vec<SharedBlockEntity>>,
}

impl BlockEntityManager {
    pub fn tick(&self) {
        let snapshot = self.tickers.lock().clone();
        for be in snapshot {
            be.advance_tick_count();
            be.tick();
        }
    }
}

#[derive(Default)]
pub struct World {
    pub entities: EntityManager,
    pub block_entities: BlockEntityManager,
    game_time: AtomicU64,
    next_id: AtomicU32,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn game_time(&self) -> u64 {
        self.game_time.load(Ordering::Relaxed)
    }

    /// Order as in Steel: time -> entities -> block entities.
    pub fn tick_game(&self) {
        self.game_time.fetch_add(1, Ordering::Relaxed);
        self.entities.tick_entities();
        self.block_entities.tick();
    }

    pub fn spawn<T: Entity>(&self, make: impl FnOnce(u32) -> T) -> Arc<T> {
        let entity = Arc::new(make(self.next_id.fetch_add(1, Ordering::Relaxed)));
        self.entities.tick_list.lock().push(entity.clone());
        entity
    }

    pub fn add_block_entity<T: BlockEntity>(&self, be: T) -> Arc<T> {
        let be = Arc::new(be);
        self.block_entities.tickers.lock().push(be.clone());
        be
    }
}
