//! `steel-ecs-bevy`: the `steel-current` entity / block-entity / tick design ported to
//! `bevy_ecs` (standalone crate, single-threaded executor, no `multi_threaded` feature).
//! Same entities, goals, block entities and tick order, same log output as `steel-current`
//! and `steel-ecs-ai`; the trait hierarchy is component sets, the tick methods are systems
//! with typed `Query` params.

pub mod block_entity;
pub mod entity;
pub mod log;
pub mod util;
pub mod world;

pub use world::GameWorld;
