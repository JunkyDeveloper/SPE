//! Steel: `block_entity/mod.rs`. Same shape as entities, smaller: a base (position, tick count),
//! optional parts (`Campfire`, `Furnace`), and a kind whose pipeline replaces `BlockEntity::tick`.
//! Concrete block entities only do counter math and log milestones.

use std::fmt;

pub type BlockStage = fn(&mut BlockEntity);

pub struct BlockKind {
    pub name: &'static str,
    pub pipeline: &'static [BlockStage],
}

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

pub struct BlockEntity {
    pub pos: (i32, i32, i32),
    pub tick_count: u32,
    pub kind: &'static BlockKind,
    pub campfire: Option<Campfire>,
    pub furnace: Option<Furnace>,
}

impl BlockEntity {
    pub fn new(kind: &'static BlockKind, pos: (i32, i32, i32)) -> Self {
        Self {
            pos,
            tick_count: 0,
            kind,
            campfire: None,
            furnace: None,
        }
    }

    pub fn campfire_mut(&mut self) -> &mut Campfire {
        self.campfire
            .as_mut()
            .expect("campfire stage on a block entity without a Campfire")
    }

    pub fn furnace_mut(&mut self) -> &mut Furnace {
        self.furnace
            .as_mut()
            .expect("furnace stage on a block entity without a Furnace")
    }

    /// `[ tick] name@x,y,z: msg`
    pub fn log(&self, msg: impl fmt::Display) {
        let (x, y, z) = self.pos;
        println!(
            "[{:>4}] {}@{x},{y},{z}: {msg}",
            self.tick_count, self.kind.name
        );
    }
}

pub static CAMPFIRE: BlockKind = BlockKind {
    name: "campfire",
    pipeline: &[tick_campfire],
};

pub static FURNACE: BlockKind = BlockKind {
    name: "furnace",
    pipeline: &[tick_furnace],
};

pub fn campfire(pos: (i32, i32, i32), cook_time: u32) -> BlockEntity {
    BlockEntity {
        campfire: Some(Campfire {
            cook_time,
            progress: 0,
            cooked: 0,
        }),
        ..BlockEntity::new(&CAMPFIRE, pos)
    }
}

pub fn furnace(pos: (i32, i32, i32), fuel: u32) -> BlockEntity {
    BlockEntity {
        furnace: Some(Furnace {
            fuel,
            progress: 0,
            output: 0,
        }),
        ..BlockEntity::new(&FURNACE, pos)
    }
}

/// Steel: `CampfireBlockEntity::tick`.
pub fn tick_campfire(be: &mut BlockEntity) {
    let fire = be.campfire_mut();
    fire.progress += 1;
    if fire.progress >= fire.cook_time {
        fire.progress = 0;
        fire.cooked += 1;
        let cooked = fire.cooked;
        be.log(format_args!("cooked #{cooked}"));
    }
}

/// Steel: `FurnaceBlockEntity::tick`.
pub fn tick_furnace(be: &mut BlockEntity) {
    let furnace = be.furnace_mut();
    if furnace.fuel == 0 {
        return;
    }
    furnace.fuel -= 1;
    furnace.progress += 1;
    let fuel = furnace.fuel;
    let mut output = None;
    if furnace.progress >= 10 {
        furnace.progress = 0;
        furnace.output += 1;
        output = Some(furnace.output);
    }
    if let Some(output) = output {
        be.log(format_args!("output {output}"));
    }
    if fuel == 0 {
        be.log("out of fuel");
    }
}
