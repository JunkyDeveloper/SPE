//! Entity trait hierarchy, same shape as Steel's `steel-core/src/entity`:
//!
//! ```text
//! Entity -> LivingEntity -> Mob
//!                       \-> AgeableMob -> Animal      (Steel: Mob -> AgeableMob -> Animal)
//! ```
//!
//! Pattern kept from Steel:
//! - one `*Base` struct per level, reached through a required accessor (`living_base()`, ...)
//! - all methods take `&self`; state lives behind mutexes
//! - `as_*` hooks for "downcasting" to a trait level
//! - concrete types chain the per-level tick steps by hand in `ai_step`
//!
//! Each level only owns one tiny piece of data and one tick step.
//! `Mob` additionally owns a `GoalSelector` that drives behaviour.

use std::sync::Arc;

use crate::util::{SyncMutex, Vec3};

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

pub type SharedEntity = Arc<dyn Entity>;

#[derive(Debug)]
pub struct EntityBaseState {
    pub tick_count: u32,
    pub position: Vec3,
    pub velocity: Vec3,
    pub removed: bool,
}

/// Steel: `EntityBase`.
pub struct EntityBase {
    id: u32,
    name: &'static str,
    state: SyncMutex<EntityBaseState>,
}

impl EntityBase {
    pub fn new(id: u32, name: &'static str, position: Vec3) -> Self {
        Self {
            id,
            name,
            state: SyncMutex::new(EntityBaseState {
                tick_count: 0,
                position,
                velocity: Vec3::ZERO,
                removed: false,
            }),
        }
    }

    pub fn with<R>(&self, f: impl FnOnce(&mut EntityBaseState) -> R) -> R {
        f(&mut self.state.lock())
    }
}

pub trait Entity: Send + Sync + 'static {
    fn base(&self) -> &EntityBase;

    // ---- data accessors ----
    fn id(&self) -> u32 {
        self.base().id
    }
    fn name(&self) -> &'static str {
        self.base().name
    }
    fn position(&self) -> Vec3 {
        self.base().with(|s| s.position)
    }
    fn set_position(&self, p: Vec3) {
        self.base().with(|s| s.position = p);
    }
    fn velocity(&self) -> Vec3 {
        self.base().with(|s| s.velocity)
    }
    fn set_velocity(&self, v: Vec3) {
        self.base().with(|s| s.velocity = v);
    }
    fn tick_count(&self) -> u32 {
        self.base().with(|s| s.tick_count)
    }
    fn advance_tick_count(&self) {
        self.base().with(|s| s.tick_count += 1);
    }
    fn is_removed(&self) -> bool {
        self.base().with(|s| s.removed)
    }
    fn set_removed(&self) {
        self.base().with(|s| s.removed = true);
    }
    fn log(&self, msg: &str) {
        println!("[{:>4}] {}#{}: {msg}", self.tick_count(), self.name(), self.id());
    }

    // ---- ticking ----

    /// Called once per game tick. Default: living entities use the living chain,
    /// everything else falls back to `default_tick`. Object entities override this.
    fn tick(&self) {
        match self.as_living_entity() {
            Some(living) => living.tick_living_entity(),
            None => self.default_tick(),
        }
    }

    fn default_tick(&self) {
        self.base_tick();
    }

    /// `position += velocity`.
    fn base_tick(&self) {
        self.base().with(|s| s.position = s.position + s.velocity);
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
}
