//! A deliberately small, std-only ECS. It exists to make the cost of "bring your own ECS"
//! visible: everything a library like bevy_ecs/hecs would give for free is written out here.
//!
//! - [`Entity`]/[`Entities`]: generational handles + allocator
//! - [`SparseSet`]: one packed storage per component type
//! - [`World`]: entities, type-erased storages, resources, runtime borrow checking
//! - [`Schedule`]: an ordered `Vec<fn(&mut World)>`
//!
//! Not provided (and therefore written by hand wherever needed): typed multi-component
//! queries, change detection, deferred commands, bundles, system parameters, parallel
//! execution, component hooks/observers.

mod entity;
mod schedule;
mod storage;
mod world;

pub use entity::{Entities, Entity};
pub use schedule::{Schedule, System};
pub use storage::{AnyStorage, SparseSet};
pub use world::World;
