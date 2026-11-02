//! The order of an entity's tick steps, written down once.
//!
//! steel-current spelled this out in `tick_living_entity` (default tick, regeneration,
//! `ai_step`) and then again in every concrete type's `ai_step` (`mob_ai_step`,
//! `tick_ageable_mob`, `tick_animal`). Here it is one list per kind of thing. Cow and chicken
//! share `ANIMAL`; a mob that differs from vanilla in one step gets its own list with that one
//! stage replaced.
//!
//! The driver does `tick_count += 1` before running a pipeline (as `tick_entities` did), so it
//! is not a stage.

use super::entities::{falling_block, item};
use super::{Stage, ageable, animal, apply_velocity, living, mob};

/// Cow, chicken: living -> mob -> ageable -> animal.
pub const ANIMAL: &[Stage] = &[
    apply_velocity,
    living::regenerate,
    mob::run_goals,
    ageable::age_up,
    animal::get_hungrier,
];

pub const ITEM: &[Stage] = &[apply_velocity, item::despawn_countdown];

pub const FALLING_BLOCK: &[Stage] = &[apply_velocity, falling_block::fall];
