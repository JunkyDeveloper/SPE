//! Steel: `block_entity/mod.rs`. Base (name, position, tick count) + a `tick` hook.
//! Concrete block entities only do counter math and log milestones.

use crate::util::SyncMutex;

pub type SharedBlockEntity = std::sync::Arc<dyn BlockEntity>;

pub struct BlockEntityBase {
    name: &'static str,
    pos: (i32, i32, i32),
    tick_count: SyncMutex<u32>,
}

impl BlockEntityBase {
    pub fn new(name: &'static str, pos: (i32, i32, i32)) -> Self {
        Self { name, pos, tick_count: SyncMutex::new(0) }
    }
}

pub trait BlockEntity: Send + Sync + 'static {
    fn base(&self) -> &BlockEntityBase;

    fn advance_tick_count(&self) {
        *self.base().tick_count.lock() += 1;
    }
    fn tick_count(&self) -> u32 {
        *self.base().tick_count.lock()
    }
    fn log(&self, msg: &str) {
        let (x, y, z) = self.base().pos;
        println!("[{:>4}] {}@{x},{y},{z}: {msg}", self.tick_count(), self.base().name);
    }
    fn tick(&self);
}

/// Cooks one item: progress += 1 each tick, done at `cook_time`, then restarts.
pub struct CampfireBlockEntity {
    base: BlockEntityBase,
    cook_time: u32,
    progress: SyncMutex<u32>,
    cooked: SyncMutex<u32>,
}

impl CampfireBlockEntity {
    pub fn new(pos: (i32, i32, i32), cook_time: u32) -> Self {
        Self {
            base: BlockEntityBase::new("campfire", pos),
            cook_time,
            progress: SyncMutex::new(0),
            cooked: SyncMutex::new(0),
        }
    }
    pub fn progress(&self) -> u32 {
        *self.progress.lock()
    }
    pub fn cooked(&self) -> u32 {
        *self.cooked.lock()
    }
}

impl BlockEntity for CampfireBlockEntity {
    fn base(&self) -> &BlockEntityBase {
        &self.base
    }
    fn tick(&self) {
        let done = {
            let mut p = self.progress.lock();
            *p += 1;
            if *p >= self.cook_time {
                *p = 0;
                true
            } else {
                false
            }
        };
        if done {
            *self.cooked.lock() += 1;
            self.log(&format!("cooked #{}", self.cooked()));
        }
    }
}

/// Burns 1 fuel/tick; every 10 burning ticks produces 1 output.
pub struct FurnaceBlockEntity {
    base: BlockEntityBase,
    fuel: SyncMutex<u32>,
    progress: SyncMutex<u32>,
    output: SyncMutex<u32>,
}

impl FurnaceBlockEntity {
    pub fn new(pos: (i32, i32, i32), fuel: u32) -> Self {
        Self {
            base: BlockEntityBase::new("furnace", pos),
            fuel: SyncMutex::new(fuel),
            progress: SyncMutex::new(0),
            output: SyncMutex::new(0),
        }
    }
    pub fn fuel(&self) -> u32 {
        *self.fuel.lock()
    }
    pub fn output(&self) -> u32 {
        *self.output.lock()
    }
}

impl BlockEntity for FurnaceBlockEntity {
    fn base(&self) -> &BlockEntityBase {
        &self.base
    }
    fn tick(&self) {
        let fuel = {
            let mut f = self.fuel.lock();
            if *f == 0 {
                return;
            }
            *f -= 1;
            *f
        };
        let produced = {
            let mut p = self.progress.lock();
            *p += 1;
            if *p >= 10 {
                *p = 0;
                true
            } else {
                false
            }
        };
        if produced {
            *self.output.lock() += 1;
            self.log(&format!("output {}", self.output()));
        }
        if fuel == 0 {
            self.log("out of fuel");
        }
    }
}
