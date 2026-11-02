//! Entities as plain data plus an ordered list of tick stages. No trait hierarchy.
//!
//! ```text
//! steel-current                           steel-ecs-ai
//! trait Entity       (EntityBase)         Entity.base: Base                 id, position, velocity, ...
//! trait LivingEntity (LivingEntityBase)   Entity.living: Option<Living>     health
//! trait Mob          (MobBase)            Entity.mob: Option<Mob>           move speed, yaw, goals
//! trait AgeableMob   (AgeableMobBase)     Entity.ageable: Option<Ageable>   age
//! trait Animal       (AnimalBase)         Entity.animal: Option<Animal>     hunger
//! tick_living_entity + ai_step chains     pipeline::ANIMAL                  one ordered list of stages
//! one struct + 5 trait impls per mob      one `AnimalSpec` value per mob
//! ```
//!
//! What a mob *is* is the parts it carries. What a tick *does* is its [`Kind`]'s pipeline: a
//! `&[Stage]` of plain `fn(&mut Entity, &mut Cx)` run in order. The vanilla step order lives in
//! [`pipeline`] once instead of being re-chained by hand in every concrete type, and a type
//! that differs from vanilla in one step swaps that one stage.
//!
//! The driver (`GameWorld::tick_game`) is the old `EntityManager::tick_entities`: loop over the
//! entities in spawn order, tick one completely (all its stages) before the next, so an entity
//! can change another one and the later one sees it in the same tick.

use std::fmt;

use crate::util::Vec3;

pub mod ageable;
pub mod animal;
pub mod entities;
pub mod goal;
pub mod living;
pub mod mob;
pub mod pipeline;
pub mod store;

pub use ageable::Ageable;
pub use animal::{Animal, AnimalSpec, spawn_animal};
pub use entities::falling_block::FallingBlock;
pub use entities::item::Item;
pub use goal::*;
pub use living::Living;
pub use mob::Mob;
pub use store::{Cx, Handle, Store};

/// Steel-style network id: monotonic, never reused, assigned when the entity is spawned.
/// (`Handle` is the world's recycled, generation-checked slot reference.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u32);

/// Steel: `EntityBase`. What every entity has.
#[derive(Clone, Copy, Debug)]
pub struct Base {
    pub id: EntityId,
    pub position: Vec3,
    pub velocity: Vec3,
    pub tick_count: u32,
    pub removed: bool,
}

/// One step of an entity's tick.
pub type Stage = fn(&mut Entity, &mut Cx);

/// The type of an entity: log name + the stages its tick runs, in order.
pub struct Kind {
    pub name: &'static str,
    pub pipeline: &'static [Stage],
}

pub struct Entity {
    pub base: Base,
    pub kind: &'static Kind,
    pub living: Option<Living>,
    pub mob: Option<Mob>,
    pub ageable: Option<Ageable>,
    pub animal: Option<Animal>,
    pub item: Option<Item>,
    pub falling_block: Option<FallingBlock>,
}

impl Entity {
    /// A bare entity. Add parts with struct-update syntax or by assigning the fields; the id is
    /// assigned by [`Cx::spawn`].
    pub fn new(kind: &'static Kind, position: Vec3) -> Self {
        Self {
            base: Base {
                id: EntityId(0),
                position,
                velocity: Vec3::ZERO,
                tick_count: 0,
                removed: false,
            },
            kind,
            living: None,
            mob: None,
            ageable: None,
            animal: None,
            item: None,
            falling_block: None,
        }
    }

    pub fn name(&self) -> &'static str {
        self.kind.name
    }

    /// `[ tick] name#id: msg`, printed immediately (entities tick one at a time, so lines come
    /// out grouped per entity without any buffering).
    pub fn log(&self, msg: impl fmt::Display) {
        println!(
            "[{:>4}] {}#{}: {msg}",
            self.base.tick_count, self.kind.name, self.base.id.0
        );
    }
}

/// `entity.living()` / `entity.living_mut()` etc.: the part, or a panic naming the missing one.
/// A pipeline stage that needs a part its entity lacks is a bug in the entity's definition.
macro_rules! part_accessors {
    ($($field:ident / $field_mut:ident: $ty:ty),* $(,)?) => {
        impl Entity {
            $(
                #[track_caller]
                pub fn $field(&self) -> &$ty {
                    self.$field.as_ref().expect(concat!(
                        "`", stringify!($field), "` stage on an entity without that part"
                    ))
                }

                #[track_caller]
                pub fn $field_mut(&mut self) -> &mut $ty {
                    self.$field.as_mut().expect(concat!(
                        "`", stringify!($field), "` stage on an entity without that part"
                    ))
                }
            )*
        }
    };
}

part_accessors! {
    living / living_mut: Living,
    mob / mob_mut: Mob,
    ageable / ageable_mut: Ageable,
    animal / animal_mut: Animal,
    item / item_mut: Item,
    falling_block / falling_block_mut: FallingBlock,
}

/// Steel: `Entity::base_tick`, `position += velocity`. First stage of every pipeline.
pub fn apply_velocity(e: &mut Entity, _cx: &mut Cx) {
    e.base.position = e.base.position + e.base.velocity;
}
