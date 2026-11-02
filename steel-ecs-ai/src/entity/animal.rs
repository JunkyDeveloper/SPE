//! Steel: `animal.rs`. Part: hunger. Stage: get hungrier up to the limit, log once when hungry.
//! Something else (a goal) has to reset it.
//!
//! [`AnimalSpec`] is what `CowEntity` / `ChickenEntity` used to be: five trait impls of mostly
//! forwarding code, now one value.

use super::{Ageable, Cx, Entity, GoalSelector, Kind, Living, Mob};
use crate::util::Vec3;

pub struct Animal {
    pub hunger: u32,
    pub hunger_limit: u32,
}

impl Animal {
    pub fn new(hunger_limit: u32) -> Self {
        Self {
            hunger: 0,
            hunger_limit,
        }
    }

    pub fn is_hungry(&self) -> bool {
        self.hunger >= self.hunger_limit
    }

    pub fn reset_hunger(&mut self) {
        self.hunger = 0;
    }
}

pub fn get_hungrier(e: &mut Entity, _cx: &mut Cx) {
    let animal = e.animal_mut();
    if animal.hunger >= animal.hunger_limit {
        return;
    }
    animal.hunger += 1;
    if animal.hunger == animal.hunger_limit {
        e.log("is hungry");
    }
}

/// Everything that differs between animals. The behavior (which stages run, in which order) is
/// the `kind`'s pipeline, shared.
pub struct AnimalSpec {
    pub kind: Kind,
    pub health: i32,
    pub max_health: i32,
    pub move_speed: f64,
    pub adult_age: u32,
    pub hunger_limit: u32,
    /// Registers this animal's goals (shared ones and its own).
    pub goals: fn(&mut GoalSelector),
}

/// `age` below `spec.adult_age` means baby.
pub fn spawn_animal(spec: &'static AnimalSpec, position: Vec3, age: u32) -> Entity {
    let mut mob = Mob::new(spec.move_speed);
    (spec.goals)(&mut mob.goals);
    Entity {
        living: Some(Living::new(spec.health, spec.max_health)),
        mob: Some(mob),
        ageable: Some(Ageable::new(age, spec.adult_age)),
        animal: Some(Animal::new(spec.hunger_limit)),
        ..Entity::new(&spec.kind, position)
    }
}
