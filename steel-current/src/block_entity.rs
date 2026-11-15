//! Steel: `block_entity/mod.rs`. Base (type, position, removed flag) + a `tick(&self, world)` hook;
//! removal goes through the blanket `BlockEntityLifecycleExt`.
//! Concrete block entities only do counter math and log milestones.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::util::{BlockPos, SyncMutex};
use crate::world::World;

pub type SharedBlockEntity = Arc<dyn BlockEntity>;

pub struct BlockEntityBase {
    block_entity_type: &'static str,
    pos: BlockPos,
    removed: AtomicBool,
}

impl BlockEntityBase {
    pub fn new(block_entity_type: &'static str, pos: BlockPos) -> Self {
        Self { block_entity_type, pos, removed: AtomicBool::new(false) }
    }
}

pub trait BlockEntity: Send + Sync + 'static {
    fn base(&self) -> &BlockEntityBase;

    fn get_type(&self) -> &'static str {
        self.base().block_entity_type
    }
    fn get_block_pos(&self) -> BlockPos {
        self.base().pos
    }
    fn log(&self, world: &World, msg: &str) {
        let (x, y, z) = self.get_block_pos();
        println!("[{:>4}] {}@{x},{y},{z}: {msg}", world.game_time(), self.get_type());
    }

    /// Called every game tick for ticking block entities.
    fn tick(&self, _world: &Arc<World>) {}
}

/// Final lifecycle operations; concrete block entities can't override these.
pub trait BlockEntityLifecycleExt: BlockEntity {
    fn is_removed(&self) -> bool {
        self.base().removed.load(Ordering::Relaxed)
    }
    fn set_removed(&self) {
        self.base().removed.store(true, Ordering::Relaxed);
    }
}

impl<T: BlockEntity + ?Sized> BlockEntityLifecycleExt for T {}

const BURN_DURATION: i32 = 20; // vanilla coal: 1600
const COOK_TIME: i32 = 10; // vanilla 200

#[derive(Default)]
pub struct FurnaceState {
    pub input: u32,
    pub fuel: u32,
    pub result: u32,
    pub lit_time: i32,
    pub cooking_progress: i32,
}

/// Vanilla `AbstractFurnaceBlockEntity.serverTick`, slots reduced to counters.
pub struct FurnaceBlockEntity {
    base: BlockEntityBase,
    state: SyncMutex<FurnaceState>,
}

impl FurnaceBlockEntity {
    pub fn new(pos: BlockPos, input: u32, fuel: u32) -> Self {
        Self { base: BlockEntityBase::new("furnace", pos), state: SyncMutex::new(FurnaceState { input, fuel, ..Default::default() }) }
    }
    pub fn state(&self) -> std::sync::MutexGuard<'_, FurnaceState> {
        self.state.lock()
    }
}

impl BlockEntity for FurnaceBlockEntity {
    fn base(&self) -> &BlockEntityBase {
        &self.base
    }

    fn tick(&self, world: &Arc<World>) {
        let burned = {
            let mut s = self.state.lock();
            if s.lit_time > 0 {
                s.lit_time -= 1;
            }
            let can_burn = s.input > 0;
            if s.lit_time <= 0 && can_burn && s.fuel > 0 {
                s.fuel -= 1;
                s.lit_time = BURN_DURATION;
            }
            if s.lit_time > 0 && can_burn {
                s.cooking_progress += 1;
            } else {
                s.cooking_progress = 0;
            }
            let burned = s.cooking_progress == COOK_TIME;
            if burned {
                s.cooking_progress = 0;
                s.input -= 1;
                s.result += 1;
            }
            burned
        };
        if burned {
            self.log(world, &format!("smelted, result x{}", self.state.lock().result));
        }
    }
}

/// Cooks one item: progress += 1 each tick, done at `cook_time`, then restarts.
pub struct CampfireBlockEntity {
    base: BlockEntityBase,
    cook_time: i32,
    progress: SyncMutex<i32>,
    cooked: SyncMutex<u32>,
}

impl CampfireBlockEntity {
    pub fn new(pos: BlockPos, cook_time: i32) -> Self {
        Self { base: BlockEntityBase::new("campfire", pos), cook_time, progress: SyncMutex::new(0), cooked: SyncMutex::new(0) }
    }
    pub fn cooked(&self) -> u32 {
        *self.cooked.lock()
    }
}

impl BlockEntity for CampfireBlockEntity {
    fn base(&self) -> &BlockEntityBase {
        &self.base
    }
    fn tick(&self, world: &Arc<World>) {
        let done = {
            let mut p = self.progress.lock();
            *p += 1;
            *p >= self.cook_time && {
                *p = 0;
                true
            }
        };
        if done {
            *self.cooked.lock() += 1;
            self.log(world, &format!("cooked #{}", self.cooked()));
        }
    }
}
