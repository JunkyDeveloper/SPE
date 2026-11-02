//! Entity trait hierarchy -> components, same mapping as steel-ecs-ai, but the components are
//! plain `#[derive(Component)]` structs and the ECS itself is `bevy_ecs`.
//!
//! ```text
//! steel-current                      steel-ecs-bevy
//! Entity        (EntityBase)         EntityId, Name, Position, Velocity, TickCount, Removed
//! LivingEntity  (LivingEntityBase)   Health
//! Mob           (MobBase)            MoveSpeed, Yaw, GoalSelector
//! AgeableMob    (AgeableMobBase)     Ageable
//! Animal        (AnimalBase)         Hunger
//! ```
//!
//! What bevy gives for free compared to the raw crate (steel-ecs-ai):
//! - a system is a plain function whose *parameters* are its queries/resources
//!   (`Query<(&EntityId, &Name, &mut Health), Without<Removed>>`); no hand-written guards,
//!   no manual joins, no `EntityHeader` helper, no per-system `Removed` check;
//! - `Bundle`s: a spawn is one `spawn(bundle)` call and works the same from `World` and
//!   from `Commands` inside a system;
//! - access conflicts between a system's own params are rejected when the schedule is
//!   initialized (not at the first overlapping borrow), and no `register` calls are needed;
//! - `Commands` for deferred spawn/despawn/insert while iterating.
//!
//! What stays the same: a mob "has" `Position` only by convention (a mob query that requires
//! it simply never matches an entity without it, so a missing component makes the entity
//! silently *not tick* instead of panicking as in steel-ecs-ai).

use bevy_ecs::prelude::{Bundle, Commands, Component, Entity, Query, Resource, With, Without};

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

/// Steel-style network id: monotonic, never reused. (`Entity` is bevy's recycled handle.)
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u32);

/// Allocator for [`EntityId`]. A resource (not a `GameWorld` field) so systems can spawn
/// through `Commands` and still get ids.
#[derive(Resource, Default)]
pub struct EntityIds {
    next: u32,
}

impl EntityIds {
    pub fn allocate(&mut self) -> EntityId {
        let id = EntityId(self.next);
        self.next += 1;
        id
    }
}

#[derive(Component)]
pub struct Name(pub &'static str);

#[derive(Component, Clone, Copy, Debug)]
pub struct Position(pub Vec3);

#[derive(Component, Clone, Copy, Debug)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct TickCount(pub u32);

/// Marker: skipped by every entity system, despawned at the end of the entity pass.
#[derive(Component)]
pub struct Removed;

/// Steel: `EntityBase`. Every entity kind starts from this bundle.
#[derive(Bundle)]
pub struct EntityBaseBundle {
    pub id: EntityId,
    pub name: Name,
    pub position: Position,
    pub velocity: Velocity,
    pub tick_count: TickCount,
}

impl EntityBaseBundle {
    pub fn new(id: EntityId, name: &'static str, position: Vec3) -> Self {
        Self {
            id,
            name: Name(name),
            position: Position(position),
            velocity: Velocity(Vec3::ZERO),
            tick_count: TickCount(0),
        }
    }
}

// ---- systems: the part of `EntityManager::tick_entities` + `Entity::tick` shared by all ----

/// Steel: `entity.advance_tick_count()`.
pub fn advance_tick_count(mut ticks: Query<&mut TickCount, Without<Removed>>) {
    for mut tick in &mut ticks {
        tick.0 += 1;
    }
}

/// Steel: `Entity::base_tick`, `position += velocity`. Runs for every entity kind because
/// living, item and falling-block ticks all start with `default_tick`.
pub fn base_tick(mut movers: Query<(&mut Position, &Velocity), Without<Removed>>) {
    for (mut position, velocity) in &mut movers {
        position.0 = position.0 + velocity.0;
    }
}

/// Steel: `EntityManager::tick_entities` final `retain(|e| !e.is_removed())`.
/// The despawn is a command, applied at the next sync point.
pub fn drop_removed(mut commands: Commands, removed: Query<Entity, With<Removed>>) {
    for entity in &removed {
        commands.entity(entity).despawn();
    }
}
