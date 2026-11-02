//! Steel: `animal.rs`. Data: hunger. Tick step: get hungrier up to the limit, log once when hungry.
//! Something else (a goal) has to reset it.

use crate::ecs::{Entity, World};
use crate::entity::{
    Ageable, EntityHeader, EntityId, Goal, GoalSelector, Health, MoveSpeed, Yaw, spawn_base,
};
use crate::log::Logs;
use crate::util::Vec3;

pub struct Hunger {
    pub value: u32,
    pub limit: u32,
}

impl Hunger {
    pub fn new(limit: u32) -> Self {
        Self { value: 0, limit }
    }

    pub fn is_hungry(&self) -> bool {
        self.value >= self.limit
    }

    pub fn reset(&mut self) {
        self.value = 0;
    }
}

pub fn tick_animals(world: &mut World) {
    let header = EntityHeader::new(world);
    let mut hungers = world.write::<Hunger>();
    let mut logs = world.res_mut::<Logs>();
    for (entity, hunger) in hungers.iter_mut() {
        if !header.is_active(entity) || hunger.value >= hunger.limit {
            continue;
        }
        hunger.value += 1;
        if hunger.value == hunger.limit {
            header.log(&mut logs, entity, "is hungry");
        }
    }
}

/// The per-type constants that were trait methods (`move_speed`, `adult_age`, `hunger_limit`)
/// and constructor arguments in steel-current. Cow and chicken only differ in this data.
pub struct AnimalSpec {
    pub name: &'static str,
    pub health: i32,
    pub max_health: i32,
    pub move_speed: f64,
    pub adult_age: u32,
    pub hunger_limit: u32,
    pub goals: Vec<(i32, Box<dyn Goal>)>,
}

/// Inserts the full Entity -> Living -> Mob -> Ageable -> Animal component chain.
pub fn spawn_animal(
    world: &mut World,
    id: EntityId,
    position: Vec3,
    age: u32,
    spec: AnimalSpec,
) -> Entity {
    let entity = spawn_base(world, id, spec.name, position);
    let mut selector = GoalSelector::default();
    for (priority, goal) in spec.goals {
        selector.add_goal(priority, goal);
    }
    world.insert(entity, Health::new(spec.health, spec.max_health));
    world.insert(entity, MoveSpeed(spec.move_speed));
    world.insert(entity, Yaw(0.0));
    world.insert(entity, selector);
    world.insert(entity, Ageable::new(age, spec.adult_age));
    world.insert(entity, Hunger::new(spec.hunger_limit));
    entity
}
