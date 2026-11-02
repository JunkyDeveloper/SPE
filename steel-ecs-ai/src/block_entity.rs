//! Steel: `block_entity/mod.rs`. Base (name, position, tick count) + a `tick` hook.
//! Concrete block entities only do counter math and log milestones.
//!
//! Block entities are ECS entities with their own base components (`BlockName`, `BlockPos`,
//! `BlockTickCount`) instead of reusing `Name`/`Position`/`TickCount`: the entity systems
//! iterate those storages and must not tick block entities (different phase, no `Removed`).
//! `Campfire` and `Furnace` are the per-kind components; their tick systems replace the
//! `BlockEntity::tick` trait method.

use std::cell::Ref;

use crate::ecs::{Entity, SparseSet, World};
use crate::log::Logs;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockEntityId(pub u32);

pub struct BlockName(pub &'static str);

#[derive(Clone, Copy, Debug)]
pub struct BlockPos(pub (i32, i32, i32));

pub struct BlockTickCount(pub u32);

/// Cooks one item: progress += 1 each tick, done at `cook_time`, then restarts.
pub struct Campfire {
    pub cook_time: u32,
    pub progress: u32,
    pub cooked: u32,
}

/// Burns 1 fuel/tick; every 10 burning ticks produces 1 output.
pub struct Furnace {
    pub fuel: u32,
    pub progress: u32,
    pub output: u32,
}

fn spawn_base(
    world: &mut World,
    id: BlockEntityId,
    name: &'static str,
    pos: (i32, i32, i32),
) -> Entity {
    let entity = world.spawn_empty();
    world.insert(entity, id);
    world.insert(entity, BlockName(name));
    world.insert(entity, BlockPos(pos));
    world.insert(entity, BlockTickCount(0));
    entity
}

pub fn spawn_campfire(
    world: &mut World,
    id: BlockEntityId,
    pos: (i32, i32, i32),
    cook_time: u32,
) -> Entity {
    let entity = spawn_base(world, id, "campfire", pos);
    world.insert(
        entity,
        Campfire {
            cook_time,
            progress: 0,
            cooked: 0,
        },
    );
    entity
}

pub fn spawn_furnace(
    world: &mut World,
    id: BlockEntityId,
    pos: (i32, i32, i32),
    fuel: u32,
) -> Entity {
    let entity = spawn_base(world, id, "furnace", pos);
    world.insert(
        entity,
        Furnace {
            fuel,
            progress: 0,
            output: 0,
        },
    );
    entity
}

/// Block-entity counterpart of `EntityHeader` (log prefix pieces).
pub struct BlockEntityHeader<'w> {
    ids: Ref<'w, SparseSet<BlockEntityId>>,
    names: Ref<'w, SparseSet<BlockName>>,
    positions: Ref<'w, SparseSet<BlockPos>>,
    ticks: Ref<'w, SparseSet<BlockTickCount>>,
}

impl<'w> BlockEntityHeader<'w> {
    pub fn new(world: &'w World) -> Self {
        Self {
            ids: world.read(),
            names: world.read(),
            positions: world.read(),
            ticks: world.read(),
        }
    }

    pub fn log(&self, logs: &mut Logs, entity: Entity, msg: &str) {
        logs.block_entity(
            *self
                .ids
                .get(entity)
                .expect("block entity without BlockEntityId"),
            self.names
                .get(entity)
                .expect("block entity without BlockName")
                .0,
            self.positions
                .get(entity)
                .expect("block entity without BlockPos")
                .0,
            self.ticks
                .get(entity)
                .expect("block entity without BlockTickCount")
                .0,
            msg,
        );
    }
}

// ---- systems: `BlockEntityManager::tick` ----

/// Steel: `be.advance_tick_count()`.
pub fn advance_block_tick_count(world: &mut World) {
    for (_, tick) in world.write::<BlockTickCount>().iter_mut() {
        tick.0 += 1;
    }
}

/// Steel: `CampfireBlockEntity::tick`.
pub fn tick_campfires(world: &mut World) {
    let header = BlockEntityHeader::new(world);
    let mut campfires = world.write::<Campfire>();
    let mut logs = world.res_mut::<Logs>();
    for (entity, fire) in campfires.iter_mut() {
        fire.progress += 1;
        if fire.progress >= fire.cook_time {
            fire.progress = 0;
            fire.cooked += 1;
            header.log(&mut logs, entity, &format!("cooked #{}", fire.cooked));
        }
    }
}

/// Steel: `FurnaceBlockEntity::tick`.
pub fn tick_furnaces(world: &mut World) {
    let header = BlockEntityHeader::new(world);
    let mut furnaces = world.write::<Furnace>();
    let mut logs = world.res_mut::<Logs>();
    for (entity, furnace) in furnaces.iter_mut() {
        if furnace.fuel == 0 {
            continue;
        }
        furnace.fuel -= 1;
        furnace.progress += 1;
        if furnace.progress >= 10 {
            furnace.progress = 0;
            furnace.output += 1;
            header.log(&mut logs, entity, &format!("output {}", furnace.output));
        }
        if furnace.fuel == 0 {
            header.log(&mut logs, entity, "out of fuel");
        }
    }
}
