//! Entity trait hierarchy -> component sets.
//!
//! ```text
//! steel-current                      steel-ecs-ai
//! Entity        (EntityBase)         EntityId, Name, Position, Velocity, TickCount, Removed
//! LivingEntity  (LivingEntityBase)   Health
//! Mob           (MobBase)            MoveSpeed, Yaw, GoalSelector
//! AgeableMob    (AgeableMobBase)     Ageable
//! Animal        (AnimalBase)         Hunger
//! ```
//!
//! What changed in the mapping:
//! - a trait level is now "has this component"; `as_living_entity()` & co. become
//!   `storage.contains(entity)`. Nothing enforces that a mob also has `Position`: that
//!   invariant moved from the type system to `expect` calls in the systems.
//! - `tick_living_entity` / `ai_step` chains written by hand in `ChickenEntity`/`CowEntity` are
//!   gone: each tick step is its own system, the schedule order replaces the call chain, and
//!   which entities take part is decided by which components they carry.
//! - a concrete entity type is just a spawn function that inserts a component set.

use std::cell::Ref;

use crate::ecs::{Entity, SparseSet, World};
use crate::log::Logs;
use crate::util::Vec3;

pub mod ageable;
pub mod animal;
pub mod entities;
pub mod goal;
pub mod living;
pub mod mob;

pub use ageable::*;
pub use animal::*;
pub use goal::*;
pub use living::*;
pub use mob::*;

/// Steel-style network id: monotonic, never reused. (`Entity::index` is recycled.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u32);

pub struct Name(pub &'static str);

#[derive(Clone, Copy, Debug)]
pub struct Position(pub Vec3);

#[derive(Clone, Copy, Debug)]
pub struct Velocity(pub Vec3);

pub struct TickCount(pub u32);

/// Marker: skipped by every entity system, despawned at the end of the entity pass.
pub struct Removed;

pub fn spawn_base(world: &mut World, id: EntityId, name: &'static str, position: Vec3) -> Entity {
    let entity = world.spawn_empty();
    world.insert(entity, id);
    world.insert(entity, Name(name));
    world.insert(entity, Position(position));
    world.insert(entity, Velocity(Vec3::ZERO));
    world.insert(entity, TickCount(0));
    entity
}

/// Steel: `Entity::set_removed`. No-op for a handle that is already gone.
pub fn mark_removed(world: &mut World, entity: Entity) {
    if world.is_alive(entity) {
        world.insert(entity, Removed);
    }
}

/// The read-only components almost every entity system needs (skip check + log prefix).
/// A hand-written stand-in for what a query tuple like `(&EntityId, &Name, &TickCount,
/// Without<Removed>)` gives for free. Holds shared guards, so a system must drop it before
/// it can `insert` a `Removed` marker.
pub struct EntityHeader<'w> {
    removed: Ref<'w, SparseSet<Removed>>,
    ids: Ref<'w, SparseSet<EntityId>>,
    names: Ref<'w, SparseSet<Name>>,
    ticks: Ref<'w, SparseSet<TickCount>>,
}

impl<'w> EntityHeader<'w> {
    pub fn new(world: &'w World) -> Self {
        Self {
            removed: world.read(),
            ids: world.read(),
            names: world.read(),
            ticks: world.read(),
        }
    }

    /// Steel: `!entity.is_removed()`.
    pub fn is_active(&self, entity: Entity) -> bool {
        !self.removed.contains(entity)
    }

    pub fn id(&self, entity: Entity) -> EntityId {
        *self.ids.get(entity).expect("entity without EntityId")
    }

    pub fn name(&self, entity: Entity) -> &'static str {
        self.names.get(entity).expect("entity without Name").0
    }

    pub fn tick_count(&self, entity: Entity) -> u32 {
        self.ticks.get(entity).expect("entity without TickCount").0
    }

    pub fn log(&self, logs: &mut Logs, entity: Entity, msg: &str) {
        logs.entity(
            self.id(entity),
            self.name(entity),
            self.tick_count(entity),
            msg,
        );
    }
}

// ---- systems: the part of `EntityManager::tick_entities` + `Entity::tick` shared by all ----

/// Steel: `entity.advance_tick_count()`.
pub fn advance_tick_count(world: &mut World) {
    let removed = world.read::<Removed>();
    let mut ticks = world.write::<TickCount>();
    for (entity, tick) in ticks.iter_mut() {
        if !removed.contains(entity) {
            tick.0 += 1;
        }
    }
}

/// Steel: `Entity::base_tick`, `position += velocity`. Runs for every entity kind because
/// living, item and falling-block ticks all start with `default_tick`.
pub fn base_tick(world: &mut World) {
    let removed = world.read::<Removed>();
    let velocities = world.read::<Velocity>();
    let mut positions = world.write::<Position>();
    for (entity, position) in positions.iter_mut() {
        if removed.contains(entity) {
            continue;
        }
        if let Some(velocity) = velocities.get(entity) {
            position.0 = position.0 + velocity.0;
        }
    }
}

/// Steel: `EntityManager::tick_entities` final `retain(|e| !e.is_removed())`.
pub fn drop_removed(world: &mut World) {
    let doomed: Vec<Entity> = world.read::<Removed>().entities().to_vec();
    for entity in doomed {
        world.despawn(entity);
    }
}
