//! Steel: `world/mod.rs` `tick_game`, `entity/manager`, `world/block_entity_ticker.rs`.
//! `EntityManager` and `BlockEntityManager` (two lists + snapshot loops) are gone: the
//! "lists" are bevy's tables and the loops are the systems in [`build_schedule`].
//!
//! # The snapshot rule ("spawned mid-tick first ticks next tick")
//!
//! steel-current ticks over a snapshot. In bevy a system spawns through `Commands`, and the
//! spawn only becomes visible at the next *sync point* (`ApplyDeferred`). By default `chain()`
//! and `before()/after()` edges **auto-insert a sync point after any system with deferred
//! params**, so an entity spawned by system A is visible to system B later in the same pass
//! and would tick in the tick it was spawned. This schedule avoids that on purpose: the whole
//! entity pass is `chain_ignore_deferred()` (no automatic sync points inside it) followed by one
//! explicit `ApplyDeferred`. Spawns issued by any system in the pass (order them with
//! `before_ignore_deferred`/`after_ignore_deferred`, *not* `before`/`after`) therefore land after
//! the last ticking system and first tick next tick, like steel-current. See
//! `tests/mid_tick_spawn.rs`, which also shows what the default `chain()` does instead.
//!
//! The same explicit sync point is what makes `Removed` (inserted via `Commands` by items and
//! falling blocks) visible to `drop_removed`.

use bevy_ecs::prelude::{Entity, IntoScheduleConfigs, ResMut, Resource, Schedule, World};
use bevy_ecs::schedule::{ApplyDeferred, SingleThreadedExecutor};

use crate::block_entity::{
    BlockEntityId, BlockEntityIds, Campfire, Furnace, advance_block_tick_count, campfire_bundle,
    furnace_bundle, tick_campfires, tick_furnaces,
};
use crate::entity::entities::falling_block::tick_falling_blocks;
use crate::entity::entities::item::tick_items;
use crate::entity::entities::{chicken, cow, falling_block, item};
use crate::entity::{
    Ageable, EntityId, EntityIds, Health, Hunger, Position, Removed, TickCount, Velocity, Yaw,
    advance_tick_count, base_tick, drop_removed, tick_ageable_mobs, tick_animals,
    tick_goal_selectors, tick_regeneration,
};
use crate::log::Logs;
use crate::util::Vec3;

/// Singleton: total ticks run (steel-current: `World::game_time`).
#[derive(Resource)]
pub struct GameTime(pub u64);

/// Order as in Steel: time -> entities -> block entities. Within the entity pass the order is
/// the one `Entity::tick` used to chain by hand: tick count, `base_tick`, regeneration,
/// `ai_step` (goals -> ageable -> animal), then the object-entity overrides.
///
/// Single-threaded on purpose: `SingleThreadedExecutor` set explicitly (it is also what
/// bevy_ecs picks when the `multi_threaded` feature is off, which it is here).
pub fn build_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(
        (
            advance_game_time,
            // ---- entities ----
            // chain_ignore_deferred: no automatic sync points inside the pass (see module docs)
            (
                advance_tick_count,
                base_tick,
                tick_regeneration,
                tick_goal_selectors,
                tick_ageable_mobs,
                tick_animals,
                tick_items,
                tick_falling_blocks,
            )
                .chain_ignore_deferred(),
            ApplyDeferred, // spawns + `Removed` markers from the pass land here
            drop_removed,
            ApplyDeferred, // despawns land here
            // ---- block entities ----
            advance_block_tick_count,
            tick_campfires,
            tick_furnaces,
            // ---- output ----
            flush_logs,
        )
            .chain_ignore_deferred(),
    );
    schedule
}

fn advance_game_time(mut time: ResMut<GameTime>) {
    time.0 += 1;
}

fn flush_logs(mut logs: ResMut<Logs>) {
    logs.flush();
}

pub struct GameWorld {
    ecs: World,
    schedule: Schedule,
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl GameWorld {
    pub fn new() -> Self {
        let mut ecs = World::new();
        ecs.insert_resource(GameTime(0));
        ecs.insert_resource(Logs::default());
        ecs.insert_resource(EntityIds::default());
        ecs.insert_resource(BlockEntityIds::default());
        Self {
            ecs,
            schedule: build_schedule(),
        }
    }

    /// Escape hatch to the raw bevy world.
    pub fn ecs(&self) -> &World {
        &self.ecs
    }

    pub fn ecs_mut(&mut self) -> &mut World {
        &mut self.ecs
    }

    /// Escape hatch to add systems (used by the mid-tick spawn test).
    pub fn schedule_mut(&mut self) -> &mut Schedule {
        &mut self.schedule
    }

    pub fn game_time(&self) -> u64 {
        self.ecs.resource::<GameTime>().0
    }

    /// Order as in Steel: time -> entities -> block entities (the first system bumps the time).
    pub fn tick_game(&mut self) {
        self.schedule.run(&mut self.ecs);
    }

    // ---- spawning ----

    fn next_entity_id(&mut self) -> EntityId {
        self.ecs.resource_mut::<EntityIds>().allocate()
    }

    fn next_block_entity_id(&mut self) -> BlockEntityId {
        self.ecs.resource_mut::<BlockEntityIds>().allocate()
    }

    pub fn spawn_cow(&mut self, position: Vec3, age: u32) -> Entity {
        let id = self.next_entity_id();
        self.ecs.spawn(cow::bundle(id, position, age)).id()
    }

    pub fn spawn_chicken(&mut self, position: Vec3, age: u32) -> Entity {
        let id = self.next_entity_id();
        self.ecs.spawn(chicken::bundle(id, position, age)).id()
    }

    pub fn spawn_item(&mut self, position: Vec3, velocity: Vec3, lifetime: u32) -> Entity {
        let id = self.next_entity_id();
        self.ecs
            .spawn(item::bundle(id, position, velocity, lifetime))
            .id()
    }

    pub fn spawn_falling_block(&mut self, x: f64, start_y: f64, z: f64, fall_speed: f64) -> Entity {
        let id = self.next_entity_id();
        self.ecs
            .spawn(falling_block::bundle(id, x, start_y, z, fall_speed))
            .id()
    }

    pub fn add_campfire(&mut self, pos: (i32, i32, i32), cook_time: u32) -> Entity {
        let id = self.next_block_entity_id();
        self.ecs.spawn(campfire_bundle(id, pos, cook_time)).id()
    }

    pub fn add_furnace(&mut self, pos: (i32, i32, i32), fuel: u32) -> Entity {
        let id = self.next_block_entity_id();
        self.ecs.spawn(furnace_bundle(id, pos, fuel)).id()
    }

    // ---- queries (steel-current: methods on `Arc<T>` / `dyn Entity`) ----

    /// Steel: `EntityManager::entities`, in spawn order. Removed-but-not-yet-dropped entities
    /// are included until the next tick, as before.
    ///
    /// `iter_entities` is the `&self` way to enumerate; `World::query` needs `&mut World`.
    pub fn entities(&self) -> Vec<Entity> {
        let mut all: Vec<(Entity, EntityId)> = self
            .ecs
            .iter_entities()
            .filter_map(|e| e.get::<EntityId>().map(|id| (e.id(), *id)))
            .collect();
        all.sort_by_key(|&(_, id)| id);
        all.into_iter().map(|(e, _)| e).collect()
    }

    /// Despawned entities count as removed.
    pub fn is_removed(&self, entity: Entity) -> bool {
        self.ecs
            .get_entity(entity)
            .map_or(true, |e| e.contains::<Removed>())
    }

    pub fn set_removed(&mut self, entity: Entity) {
        if let Ok(mut e) = self.ecs.get_entity_mut(entity) {
            e.insert(Removed);
        }
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
