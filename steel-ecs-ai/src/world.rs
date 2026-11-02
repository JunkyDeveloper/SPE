//! Steel: `world/mod.rs` `tick_game`, `entity/manager`, `world/block_entity_ticker.rs`.
//! `EntityManager` and `BlockEntityManager` (two lists + snapshot loops) are gone: the
//! "lists" are the component storages and the loops are the systems in [`build_schedule`].
//!
//! Semantics kept from steel-current: removed entities are skipped (`Without<Removed>` checks in
//! every system) and dropped after the entity pass (`drop_removed`).
//!
//! Semantics that do not carry over: steel-current ticks over a snapshot so that "things spawned
//! mid-tick first tick next tick". Here `spawn_*` needs `&mut GameWorld`, so nothing can be
//! spawned while a tick runs; a system that wants to spawn would need a deferred-command queue
//! applied after the pass, which this crate does not have.

use crate::block_entity::{
    BlockEntityId, BlockName, BlockPos, BlockTickCount, Campfire, Furnace,
    advance_block_tick_count, spawn_campfire, spawn_furnace, tick_campfires, tick_furnaces,
};
use crate::ecs::{Entity, Schedule, World};
use crate::entity::entities::falling_block::{FallingBlock, tick_falling_blocks};
use crate::entity::entities::item::{Item, tick_items};
use crate::entity::entities::{chicken, cow, falling_block, item};
use crate::entity::{
    Ageable, EntityId, GoalSelector, Health, Hunger, MoveSpeed, Name, Position, Removed, TickCount,
    Velocity, Yaw, advance_tick_count, base_tick, drop_removed, mark_removed, tick_ageable_mobs,
    tick_animals, tick_goal_selectors, tick_regeneration,
};
use crate::log::Logs;
use crate::util::Vec3;

/// Singleton: total ticks run (steel-current: `World::game_time`).
pub struct GameTime(pub u64);

fn register_components(world: &mut World) {
    world
        // entity base
        .register::<EntityId>()
        .register::<Name>()
        .register::<Position>()
        .register::<Velocity>()
        .register::<TickCount>()
        .register::<Removed>()
        // living / mob / ageable / animal
        .register::<Health>()
        .register::<MoveSpeed>()
        .register::<Yaw>()
        .register::<GoalSelector>()
        .register::<Ageable>()
        .register::<Hunger>()
        // object entities
        .register::<Item>()
        .register::<FallingBlock>()
        // block entities
        .register::<BlockEntityId>()
        .register::<BlockName>()
        .register::<BlockPos>()
        .register::<BlockTickCount>()
        .register::<Campfire>()
        .register::<Furnace>();
}

/// Order as in Steel: entities -> block entities (game time is bumped in `tick_game`).
/// Within the entity pass the order is the one `Entity::tick` used to chain by hand:
/// tick count, `base_tick`, regeneration, `ai_step` (goals -> ageable -> animal), then the
/// object-entity overrides.
fn build_schedule() -> Schedule {
    let mut schedule = Schedule::new();
    schedule
        // ---- entities ----
        .add_system(advance_tick_count)
        .add_system(base_tick)
        .add_system(tick_regeneration)
        .add_system(tick_goal_selectors)
        .add_system(tick_ageable_mobs)
        .add_system(tick_animals)
        .add_system(tick_items)
        .add_system(tick_falling_blocks)
        .add_system(drop_removed)
        // ---- block entities ----
        .add_system(advance_block_tick_count)
        .add_system(tick_campfires)
        .add_system(tick_furnaces)
        // ---- output ----
        .add_system(flush_logs);
    schedule
}

fn flush_logs(world: &mut World) {
    world.res_mut::<Logs>().flush();
}

pub struct GameWorld {
    ecs: World,
    schedule: Schedule,
    next_entity_id: u32,
    next_block_entity_id: u32,
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl GameWorld {
    pub fn new() -> Self {
        let mut ecs = World::new();
        register_components(&mut ecs);
        ecs.insert_resource(GameTime(0));
        ecs.insert_resource(Logs::default());
        Self {
            ecs,
            schedule: build_schedule(),
            next_entity_id: 0,
            next_block_entity_id: 0,
        }
    }

    /// Escape hatch to the raw ECS world.
    pub fn ecs(&self) -> &World {
        &self.ecs
    }

    pub fn ecs_mut(&mut self) -> &mut World {
        &mut self.ecs
    }

    pub fn game_time(&self) -> u64 {
        self.ecs.res::<GameTime>().0
    }

    /// Order as in Steel: time -> entities -> block entities.
    pub fn tick_game(&mut self) {
        self.ecs.res_mut::<GameTime>().0 += 1;
        self.schedule.run(&mut self.ecs);
    }

    // ---- spawning ----

    fn next_entity_id(&mut self) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id += 1;
        id
    }

    fn next_block_entity_id(&mut self) -> BlockEntityId {
        let id = BlockEntityId(self.next_block_entity_id);
        self.next_block_entity_id += 1;
        id
    }

    pub fn spawn_cow(&mut self, position: Vec3, age: u32) -> Entity {
        let id = self.next_entity_id();
        cow::spawn(&mut self.ecs, id, position, age)
    }

    pub fn spawn_chicken(&mut self, position: Vec3, age: u32) -> Entity {
        let id = self.next_entity_id();
        chicken::spawn(&mut self.ecs, id, position, age)
    }

    pub fn spawn_item(&mut self, position: Vec3, velocity: Vec3, lifetime: u32) -> Entity {
        let id = self.next_entity_id();
        item::spawn(&mut self.ecs, id, position, velocity, lifetime)
    }

    pub fn spawn_falling_block(&mut self, x: f64, start_y: f64, z: f64, fall_speed: f64) -> Entity {
        let id = self.next_entity_id();
        falling_block::spawn(&mut self.ecs, id, x, start_y, z, fall_speed)
    }

    pub fn add_campfire(&mut self, pos: (i32, i32, i32), cook_time: u32) -> Entity {
        let id = self.next_block_entity_id();
        spawn_campfire(&mut self.ecs, id, pos, cook_time)
    }

    pub fn add_furnace(&mut self, pos: (i32, i32, i32), fuel: u32) -> Entity {
        let id = self.next_block_entity_id();
        spawn_furnace(&mut self.ecs, id, pos, fuel)
    }

    // ---- queries (steel-current: methods on `Arc<T>` / `dyn Entity`) ----

    /// Steel: `EntityManager::entities`, in spawn order. Removed-but-not-yet-dropped entities
    /// are included until the next tick, as before.
    pub fn entities(&self) -> Vec<Entity> {
        let ids = self.ecs.read::<EntityId>();
        let mut all: Vec<(Entity, EntityId)> = ids.iter().map(|(e, id)| (e, *id)).collect();
        all.sort_by_key(|&(_, id)| id);
        all.into_iter().map(|(e, _)| e).collect()
    }

    /// Despawned entities count as removed.
    pub fn is_removed(&self, entity: Entity) -> bool {
        !self.ecs.is_alive(entity) || self.ecs.read::<Removed>().contains(entity)
    }

    pub fn set_removed(&mut self, entity: Entity) {
        mark_removed(&mut self.ecs, entity);
    }

    pub fn position(&self, entity: Entity) -> Option<Vec3> {
        self.ecs.get::<Position>(entity).map(|p| p.0)
    }

    pub fn velocity(&self, entity: Entity) -> Option<Vec3> {
        self.ecs.get::<Velocity>(entity).map(|v| v.0)
    }

    pub fn tick_count(&self, entity: Entity) -> Option<u32> {
        self.ecs.get::<TickCount>(entity).map(|t| t.0)
    }

    pub fn health(&self, entity: Entity) -> Option<i32> {
        self.ecs.get::<Health>(entity).map(|h| h.current)
    }

    pub fn is_baby(&self, entity: Entity) -> Option<bool> {
        self.ecs.get::<Ageable>(entity).map(|a| a.is_baby())
    }

    pub fn hunger(&self, entity: Entity) -> Option<u32> {
        self.ecs.get::<Hunger>(entity).map(|h| h.value)
    }

    pub fn is_hungry(&self, entity: Entity) -> Option<bool> {
        self.ecs.get::<Hunger>(entity).map(|h| h.is_hungry())
    }

    pub fn yaw(&self, entity: Entity) -> Option<f32> {
        self.ecs.get::<Yaw>(entity).map(|y| y.0)
    }

    pub fn campfire_cooked(&self, entity: Entity) -> Option<u32> {
        self.ecs.get::<Campfire>(entity).map(|c| c.cooked)
    }

    pub fn furnace_output(&self, entity: Entity) -> Option<u32> {
        self.ecs.get::<Furnace>(entity).map(|f| f.output)
    }

    pub fn furnace_fuel(&self, entity: Entity) -> Option<u32> {
        self.ecs.get::<Furnace>(entity).map(|f| f.fuel)
    }
}
