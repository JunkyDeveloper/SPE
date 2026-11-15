//! Entity trait hierarchy, same shape as Steel's `steel-core/src/entity`:
//!
//! ```text
//! Entity -> LivingEntity -> Mob -> PathfinderMob
//!                              \-> AgeableMob -> Animal
//! ```
//!
//! Call schema kept from Steel (bodies reduced to a few lines each):
//!
//! ```text
//! EntityManager::tick_entities
//!   advance_tick_count(); Arc::clone(e).tick()
//! Entity::tick(self: Arc<Self>)                 non-living types override this
//!   -> LivingEntity::tick_living_entity(&entity)
//!        default_tick -> base_tick              concrete mobs: base_tick = Mob::base_tick_mob
//!            base_tick_mob -> base_tick_living_entity -> entity_base_tick (fire)
//!                          -> mob_base_tick (ambient sound)
//!        tick_death / tick_mob_effects
//!        ai_step(&entity)                       concrete: mob_ai_step + tick_ageable_mob + tick_animal_love
//!            mob_ai_step -> default_ai_step
//!                server_ai_step -> mob_server_ai_step
//!                    tick_goal_selectors        -> PathfinderMob::tick_pathfinder_goal_selectors
//!                    tick_path_navigation       (navigation -> move control)
//!                    custom_server_ai_step      -> Animal::custom_server_ai_step_animal
//!                    tick_move_control / tick_look_control
//!                travel(input)                  position += input
//! ```
//!
//! Pattern kept from Steel:
//! - one `*Base` struct per level, reached through a required accessor (`living_base()`, ...)
//! - all methods take `&self`; state lives behind mutexes
//! - `as_*` hooks for "downcasting" to a trait level
//! - concrete types chain the per-level steps by hand (`base_tick`, `ai_step`, `server_ai_step`, ...)

use std::sync::{Arc, MutexGuard, OnceLock, Weak};

use crate::util::{SyncMutex, Vec3};
use crate::world::World;

pub mod ageable;
pub mod animal;
pub mod entities;
pub mod goal;
pub mod goals;
pub mod living;
pub mod mob;

pub use ageable::*;
pub use animal::*;
pub use goal::*;
pub use goals::*;
pub use living::*;
pub use mob::*;

use entities::item::ItemEntity;

pub type SharedEntity = Arc<dyn Entity>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemovalReason {
    Killed,
    Discarded,
}

/// Steel: `entity/damage`. Only the damage type and the attacking entity are kept.
#[derive(Clone, Copy, Debug)]
pub struct DamageSource {
    pub kind: &'static str,
    pub attacker: Option<i32>,
}

impl DamageSource {
    pub const GENERIC: Self = Self { kind: "generic", attacker: None };
    pub const ON_FIRE: Self = Self { kind: "on_fire", attacker: None };
    pub const MAGIC: Self = Self { kind: "magic", attacker: None };
    pub const fn mob_attack(attacker: i32) -> Self {
        Self { kind: "mob_attack", attacker: Some(attacker) }
    }
}

#[derive(Debug)]
pub struct EntityState {
    pub tick_count: i32,
    pub position: Vec3,
    pub remaining_fire_ticks: i32,
    pub removed: Option<RemovalReason>,
}

/// Steel: `EntityBase`.
pub struct EntityBase {
    id: i32,
    entity_type: &'static str,
    level: OnceLock<Weak<World>>,
    state: SyncMutex<EntityState>,
}

impl EntityBase {
    pub fn new(id: i32, entity_type: &'static str, position: Vec3) -> Self {
        Self {
            id,
            entity_type,
            level: OnceLock::new(),
            state: SyncMutex::new(EntityState {
                tick_count: 0,
                position,
                remaining_fire_ticks: 0,
                removed: None,
            }),
        }
    }

    pub fn state(&self) -> MutexGuard<'_, EntityState> {
        self.state.lock()
    }

    pub(crate) fn set_level(&self, world: Weak<World>) {
        let _ = self.level.set(world);
    }
}

/// Steel: `into_shared_entity`, lets the default `tick(self: Arc<Self>)` hand out a `SharedEntity`.
pub trait IntoSharedEntity {
    fn into_shared_entity(self: Arc<Self>) -> SharedEntity;
}

impl<T: Entity> IntoSharedEntity for T {
    fn into_shared_entity(self: Arc<Self>) -> SharedEntity {
        self
    }
}

pub trait Entity: IntoSharedEntity + Send + Sync + 'static {
    fn base(&self) -> &EntityBase;

    // ---- data accessors ----
    fn id(&self) -> i32 {
        self.base().id
    }
    fn entity_type(&self) -> &'static str {
        self.base().entity_type
    }
    fn level(&self) -> Option<Arc<World>> {
        self.base().level.get()?.upgrade()
    }
    fn position(&self) -> Vec3 {
        self.base().state().position
    }
    fn set_position(&self, p: Vec3) {
        self.base().state().position = p;
    }
    fn tick_count(&self) -> i32 {
        self.base().state().tick_count
    }
    fn advance_tick_count(&self) {
        self.base().state().tick_count += 1;
    }
    fn is_removed(&self) -> bool {
        self.base().state().removed.is_some()
    }
    fn removal_reason(&self) -> Option<RemovalReason> {
        self.base().state().removed
    }
    fn set_removed(&self, reason: RemovalReason) {
        self.base().state().removed.get_or_insert(reason);
    }
    fn discard(&self) {
        self.set_removed(RemovalReason::Discarded);
    }
    fn is_alive(&self) -> bool {
        !self.is_removed()
    }
    fn log(&self, msg: &str) {
        println!("[{:>4}] {}#{}: {msg}", self.tick_count(), self.entity_type(), self.id());
    }

    // ---- fire ----
    fn remaining_fire_ticks(&self) -> i32 {
        self.base().state().remaining_fire_ticks
    }
    fn is_on_fire(&self) -> bool {
        self.remaining_fire_ticks() > 0
    }
    fn fire_immune(&self) -> bool {
        false
    }
    fn ignite_for_ticks(&self, ticks: i32) {
        let mut s = self.base().state();
        s.remaining_fire_ticks = s.remaining_fire_ticks.max(ticks);
    }

    // ---- ticking ----

    /// Called once per game tick. The default dispatches living behaviour;
    /// non-living entities with custom tick behaviour override this directly.
    fn tick(self: Arc<Self>) {
        let entity = self.into_shared_entity();
        if let Some(living) = entity.as_living_entity() {
            living.tick_living_entity(&entity);
        }
    }

    /// Concrete ticks that mirror vanilla `super.tick()` call this.
    fn default_tick(&self) {
        self.base_tick();
    }

    /// Overridden by mobs to run their owner chain (`Mob::base_tick_mob`).
    fn base_tick(&self) {
        self.entity_base_tick();
    }

    /// Vanilla `Entity.baseTick`: burn while on fire, 1 damage every 20 ticks.
    fn entity_base_tick(&self) {
        let burn = {
            let mut s = self.base().state();
            if s.remaining_fire_ticks > 0 {
                s.remaining_fire_ticks -= 1;
                s.remaining_fire_ticks % 20 == 0
            } else {
                false
            }
        };
        if burn && !self.fire_immune() {
            self.hurt(DamageSource::ON_FIRE, 1.0);
        }
    }

    /// Returns whether the damage was applied.
    fn hurt(&self, source: DamageSource, amount: f32) -> bool {
        self.as_living_entity().is_some_and(|l| l.hurt_server(source, amount))
    }

    // ---- downcasts ----
    fn as_living_entity(&self) -> Option<&dyn LivingEntity> {
        None
    }
    fn as_mob(&self) -> Option<&dyn Mob> {
        None
    }
    fn as_ageable_mob(&self) -> Option<&dyn AgeableMob> {
        None
    }
    fn as_animal(&self) -> Option<&dyn Animal> {
        None
    }
    fn as_item_entity(&self) -> Option<&ItemEntity> {
        None
    }
}
