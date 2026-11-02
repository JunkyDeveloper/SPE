//! Steel: `block_entity/mod.rs`. Base (name, position, tick count) + a `tick` hook.
//! Concrete block entities only do counter math and log milestones.
//!
//! Block entities are bevy entities with their own base components (`BlockName`, `BlockPos`,
//! `BlockTickCount`) instead of reusing `Name`/`Position`/`TickCount`, same as steel-ecs-ai:
//! the entity systems query those components and must not tick block entities (different
//! phase, no `Removed`). `Campfire` and `Furnace` are the per-kind components; their tick
//! systems replace the `BlockEntity::tick` trait method.

use bevy_ecs::prelude::{Bundle, Component, Query, ResMut, Resource};

use crate::log::Logs;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockEntityId(pub u32);

/// Allocator for [`BlockEntityId`] (insertion order, used to order log lines).
#[derive(Resource, Default)]
pub struct BlockEntityIds {
    next: u32,
}

impl BlockEntityIds {
    pub fn allocate(&mut self) -> BlockEntityId {
        let id = BlockEntityId(self.next);
        self.next += 1;
        id
    }
}

#[derive(Component)]
pub struct BlockName(pub &'static str);

#[derive(Component, Clone, Copy, Debug)]
pub struct BlockPos(pub (i32, i32, i32));

#[derive(Component)]
pub struct BlockTickCount(pub u32);

/// Cooks one item: progress += 1 each tick, done at `cook_time`, then restarts.
#[derive(Component)]
pub struct Campfire {
    pub cook_time: u32,
    pub progress: u32,
    pub cooked: u32,
}

/// Burns 1 fuel/tick; every 10 burning ticks produces 1 output.
#[derive(Component)]
pub struct Furnace {
    pub fuel: u32,
    pub progress: u32,
    pub output: u32,
}

#[derive(Bundle)]
pub struct BlockEntityBaseBundle {
    pub id: BlockEntityId,
    pub name: BlockName,
    pub pos: BlockPos,
    pub tick_count: BlockTickCount,
}

impl BlockEntityBaseBundle {
    pub fn new(id: BlockEntityId, name: &'static str, pos: (i32, i32, i32)) -> Self {
        Self {
            id,
            name: BlockName(name),
            pos: BlockPos(pos),
            tick_count: BlockTickCount(0),
        }
    }
}

#[derive(Bundle)]
pub struct CampfireBundle {
    pub base: BlockEntityBaseBundle,
    pub campfire: Campfire,
}

#[derive(Bundle)]
pub struct FurnaceBundle {
    pub base: BlockEntityBaseBundle,
    pub furnace: Furnace,
}

pub fn campfire_bundle(id: BlockEntityId, pos: (i32, i32, i32), cook_time: u32) -> CampfireBundle {
    CampfireBundle {
        base: BlockEntityBaseBundle::new(id, "campfire", pos),
        campfire: Campfire {
            cook_time,
            progress: 0,
            cooked: 0,
        },
    }
}

pub fn furnace_bundle(id: BlockEntityId, pos: (i32, i32, i32), fuel: u32) -> FurnaceBundle {
    FurnaceBundle {
        base: BlockEntityBaseBundle::new(id, "furnace", pos),
        furnace: Furnace {
            fuel,
            progress: 0,
            output: 0,
        },
    }
}

// ---- systems: `BlockEntityManager::tick` ----

/// Steel: `be.advance_tick_count()`.
pub fn advance_block_tick_count(mut ticks: Query<&mut BlockTickCount>) {
    for mut tick in &mut ticks {
        tick.0 += 1;
    }
}

/// Steel: `CampfireBlockEntity::tick`.
pub fn tick_campfires(
    mut logs: ResMut<Logs>,
    mut campfires: Query<(
        &BlockEntityId,
        &BlockName,
        &BlockPos,
        &BlockTickCount,
        &mut Campfire,
    )>,
) {
    for (id, name, pos, tick, mut fire) in &mut campfires {
        fire.progress += 1;
        if fire.progress >= fire.cook_time {
            fire.progress = 0;
            fire.cooked += 1;
            let msg = format!("cooked #{}", fire.cooked);
            logs.block_entity(*id, name.0, pos.0, tick.0, &msg);
        }
    }
}

/// Steel: `FurnaceBlockEntity::tick`.
pub fn tick_furnaces(
    mut logs: ResMut<Logs>,
    mut furnaces: Query<(
        &BlockEntityId,
        &BlockName,
        &BlockPos,
        &BlockTickCount,
        &mut Furnace,
    )>,
) {
    for (id, name, pos, tick, mut furnace) in &mut furnaces {
        if furnace.fuel == 0 {
            continue;
        }
        furnace.fuel -= 1;
        furnace.progress += 1;
        if furnace.progress >= 10 {
            furnace.progress = 0;
            furnace.output += 1;
            let msg = format!("output {}", furnace.output);
            logs.block_entity(*id, name.0, pos.0, tick.0, &msg);
        }
        if furnace.fuel == 0 {
            logs.block_entity(*id, name.0, pos.0, tick.0, "out of fuel");
        }
    }
}
