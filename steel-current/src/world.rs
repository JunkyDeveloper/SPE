//! Minimal host for entities: two lists and a tick. Not a model of Steel's world — just enough
//! for the tick chain (Steel order: time -> entities -> block entities) and for goals that look
//! up other entities. Ticks run over a snapshot (things spawned mid-tick first tick next tick);
//! removed entities are dropped after the pass.

use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};

use crate::block_entity::{BlockEntity, BlockEntityLifecycleExt, SharedBlockEntity};
use crate::entity::{Entity, SharedEntity};
use crate::util::{SyncMutex, Vec3};

#[derive(Default)]
pub struct World {
    entities: SyncMutex<Vec<SharedEntity>>,
    block_entities: SyncMutex<Vec<SharedBlockEntity>>,
    game_time: AtomicU64,
    next_entity_id: AtomicI32,
}

impl World {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn game_time(&self) -> u64 {
        self.game_time.load(Ordering::Relaxed)
    }

    pub fn tick_game(self: &Arc<Self>) {
        self.game_time.fetch_add(1, Ordering::Relaxed);

        // Steel: `EntityManager::tick_entities`.
        for entity in self.entities() {
            if entity.is_removed() {
                continue;
            }
            entity.advance_tick_count();
            entity.tick();
        }
        self.entities.lock().retain(|e| !e.is_removed());

        // Steel: `BlockEntityTickers::tick`.
        let block_entities = self.block_entities.lock().clone();
        for be in block_entities.iter().filter(|be| !be.is_removed()) {
            be.tick(self);
        }
        self.block_entities.lock().retain(|be| !be.is_removed());
    }

    pub fn next_entity_id(&self) -> i32 {
        self.next_entity_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn add_entity(self: &Arc<Self>, entity: SharedEntity) {
        entity.base().set_level(Arc::downgrade(self));
        self.entities.lock().push(entity);
    }

    pub fn spawn<T: Entity>(self: &Arc<Self>, make: impl FnOnce(i32) -> T) -> Arc<T> {
        let entity = Arc::new(make(self.next_entity_id()));
        self.add_entity(entity.clone());
        entity
    }

    pub fn add_block_entity<T: BlockEntity>(&self, be: T) -> Arc<T> {
        let be = Arc::new(be);
        self.block_entities.lock().push(be.clone());
        be
    }

    pub fn entities(&self) -> Vec<SharedEntity> {
        self.entities.lock().clone()
    }

    pub fn get_entity_by_id(&self, id: i32) -> Option<SharedEntity> {
        self.entities().into_iter().find(|e| e.id() == id && !e.is_removed())
    }

    pub fn entities_near(&self, pos: Vec3, radius: f64) -> Vec<SharedEntity> {
        self.entities()
            .into_iter()
            .filter(|e| !e.is_removed() && e.position().distance(pos) <= radius)
            .collect()
    }
}
