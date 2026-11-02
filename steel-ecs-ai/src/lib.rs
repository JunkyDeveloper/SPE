//! `steel-ecs-ai`: the `steel-current` entity / block-entity / tick design ported to a small,
//! hand-written, std-only ECS (see [`ecs`]). Same entities, goals, block entities and tick
//! order, same log output; the trait hierarchy is replaced by component sets and the
//! per-entity tick methods by systems over component storages.

pub mod block_entity;
pub mod ecs;
pub mod entity;
pub mod log;
pub mod util;
pub mod world;

pub use world::GameWorld;
