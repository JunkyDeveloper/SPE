//! Steel: `world/mod.rs` `tick_game`, `entity/manager`, `world/block_entity_ticker.rs`.
//!
//! The two managers and their `Mutex<Vec<Arc<dyn ..>>>` lists are a [`Store`] and a `Vec`; the
//! loops are the same as before:
//!
//! - entities tick one at a time in spawn order, each through its whole pipeline;
//! - the loop runs over the entities that existed when the tick started, so entities spawned
//!   mid-tick first tick next tick;
//! - removed entities are skipped, and dropped after the pass.
//!
//! Order as in Steel: time -> entities -> block entities.

use crate::block_entity::{self, BlockEntity};
use crate::entity::entities::{chicken, cow, falling_block, item};
use crate::entity::{Cx, Entity, EntityId, Handle, Store};
use crate::util::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockEntityHandle(usize);

#[derive(Default)]
pub struct GameWorld {
    time: u64,
    next_entity_id: u32,
    store: Store,
    block_entities: Vec<BlockEntity>,
}

impl GameWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn game_time(&self) -> u64 {
        self.time
    }

    /// A context for code that runs outside a tick: spawn scripts, tests.
    pub fn cx(&mut self) -> Cx<'_> {
        Cx {
            time: self.time,
            store: &mut self.store,
            next_id: &mut self.next_entity_id,
        }
    }

    /// Order as in Steel: time -> entities -> block entities.
    pub fn tick_game(&mut self) {
        self.time += 1;
        self.tick_entities();
        self.tick_block_entities();
    }

    /// Steel: `EntityManager::tick_entities`.
    fn tick_entities(&mut self) {
        let count = self.store.len(); // entities spawned from here on first tick next tick
        for index in 0..count {
            let handle = self.store.handles()[index];
            let Some(mut entity) = self.store.take(handle) else {
                continue;
            };
            if !entity.base.removed {
                entity.base.tick_count += 1;
                let mut cx = Cx {
                    time: self.time,
                    store: &mut self.store,
                    next_id: &mut self.next_entity_id,
                };
                for stage in entity.kind.pipeline {
                    stage(&mut entity, &mut cx);
                }
            }
            self.store.put(handle, entity);
        }
        self.store.drop_removed();
    }

    /// Steel: `BlockEntityManager::tick`.
    fn tick_block_entities(&mut self) {
        for be in &mut self.block_entities {
            be.tick_count += 1;
            for stage in be.kind.pipeline {
                stage(be);
            }
        }
    }

    // ---- spawning ----

    pub fn spawn(&mut self, entity: Entity) -> Handle {
        self.cx().spawn(entity)
    }

    pub fn spawn_cow(&mut self, position: Vec3, age: u32) -> Handle {
        self.spawn(cow::spawn(position, age))
    }

    pub fn spawn_chicken(&mut self, position: Vec3, age: u32) -> Handle {
        self.spawn(chicken::spawn(position, age))
    }

    pub fn spawn_item(&mut self, position: Vec3, velocity: Vec3, lifetime: u32) -> Handle {
        self.spawn(item::spawn(position, velocity, lifetime))
    }

    pub fn spawn_falling_block(&mut self, x: f64, start_y: f64, z: f64, fall_speed: f64) -> Handle {
        self.spawn(falling_block::spawn(x, start_y, z, fall_speed))
    }

    pub fn add_block_entity(&mut self, be: BlockEntity) -> BlockEntityHandle {
        self.block_entities.push(be);
        BlockEntityHandle(self.block_entities.len() - 1)
    }

    pub fn add_campfire(&mut self, pos: (i32, i32, i32), cook_time: u32) -> BlockEntityHandle {
        self.add_block_entity(block_entity::campfire(pos, cook_time))
    }

    pub fn add_furnace(&mut self, pos: (i32, i32, i32), fuel: u32) -> BlockEntityHandle {
        self.add_block_entity(block_entity::furnace(pos, fuel))
    }

    // ---- queries (steel-current: methods on `Arc<T>` / `dyn Entity`) ----

    /// Steel: `EntityManager::entities`, in spawn order. Removed-but-not-yet-dropped entities
    /// are included until the next tick, as before.
    pub fn entities(&self) -> Vec<Handle> {
        self.store.handles().to_vec()
    }

    pub fn entity(&self, handle: Handle) -> Option<&Entity> {
        self.store.get(handle)
    }

    pub fn entity_mut(&mut self, handle: Handle) -> Option<&mut Entity> {
        self.store.get_mut(handle)
    }

    pub fn block_entity(&self, handle: BlockEntityHandle) -> &BlockEntity {
        &self.block_entities[handle.0]
    }

    /// Despawned entities count as removed.
    pub fn is_removed(&self, handle: Handle) -> bool {
        self.entity(handle).is_none_or(|e| e.base.removed)
    }

    pub fn set_removed(&mut self, handle: Handle) {
        if let Some(entity) = self.entity_mut(handle) {
            entity.base.removed = true;
        }
    }

    pub fn id(&self, handle: Handle) -> Option<EntityId> {
        self.entity(handle).map(|e| e.base.id)
    }

    pub fn position(&self, handle: Handle) -> Option<Vec3> {
        self.entity(handle).map(|e| e.base.position)
    }

    pub fn velocity(&self, handle: Handle) -> Option<Vec3> {
        self.entity(handle).map(|e| e.base.velocity)
    }

    pub fn tick_count(&self, handle: Handle) -> Option<u32> {
        self.entity(handle).map(|e| e.base.tick_count)
    }

    pub fn health(&self, handle: Handle) -> Option<i32> {
        self.entity(handle)?.living.as_ref().map(|l| l.health)
    }

    pub fn is_baby(&self, handle: Handle) -> Option<bool> {
        self.entity(handle)?.ageable.as_ref().map(|a| a.is_baby())
    }

    pub fn hunger(&self, handle: Handle) -> Option<u32> {
        self.entity(handle)?.animal.as_ref().map(|a| a.hunger)
    }

    pub fn is_hungry(&self, handle: Handle) -> Option<bool> {
        self.entity(handle)?.animal.as_ref().map(|a| a.is_hungry())
    }

    pub fn yaw(&self, handle: Handle) -> Option<f32> {
        self.entity(handle)?.mob.as_ref().map(|m| m.yaw)
    }

    pub fn campfire_cooked(&self, handle: BlockEntityHandle) -> Option<u32> {
        self.block_entity(handle)
            .campfire
            .as_ref()
            .map(|c| c.cooked)
    }

    pub fn furnace_output(&self, handle: BlockEntityHandle) -> Option<u32> {
        self.block_entity(handle).furnace.as_ref().map(|f| f.output)
    }

    pub fn furnace_fuel(&self, handle: BlockEntityHandle) -> Option<u32> {
        self.block_entity(handle).furnace.as_ref().map(|f| f.fuel)
    }
}
