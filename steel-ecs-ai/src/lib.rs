//! `steel-ecs-ai`: `steel-current` redesigned without a trait hierarchy.
//!
//! The crate name is left over from the first attempt (a hand-written ECS). There is no ECS in
//! here any more: an entity is one plain struct made of optional *parts* (`Living`, `Mob`,
//! `Ageable`, `Animal`, ...), and what a tick does is an ordered list of plain functions
//! (`entity::pipeline`). Ticking stays entity-by-entity, exactly like Steel, so vanilla's
//! tick order (and effects of one entity on the next) is preserved.

pub mod block_entity;
pub mod entity;
pub mod util;
pub mod world;

pub use world::GameWorld;
