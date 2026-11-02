//! Steel: `animal.rs`. Data: hunger. Tick step: get hungrier up to the limit, log once when hungry.
//! Something else (a goal) has to reset it.

use bevy_ecs::prelude::{Bundle, Component, Query, ResMut, Without};

use crate::entity::{
    Ageable, EntityBaseBundle, EntityId, Goal, GoalSelector, Health, MoveSpeed, Name, Removed,
    TickCount, Yaw,
};
use crate::log::Logs;
use crate::util::Vec3;

#[derive(Component)]
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

pub fn tick_animals(
    mut logs: ResMut<Logs>,
    mut animals: Query<(&EntityId, &Name, &TickCount, &mut Hunger), Without<Removed>>,
) {
    for (id, name, tick, mut hunger) in &mut animals {
        if hunger.value >= hunger.limit {
            continue;
        }
        hunger.value += 1;
        if hunger.value == hunger.limit {
            logs.entity(*id, name.0, tick.0, "is hungry");
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

/// The full Entity -> Living -> Mob -> Ageable -> Animal component chain as one bundle.
/// Works with `World::spawn` and `Commands::spawn` alike.
#[derive(Bundle)]
pub struct AnimalBundle {
    pub base: EntityBaseBundle,
    pub health: Health,
    pub move_speed: MoveSpeed,
    pub yaw: Yaw,
    pub goals: GoalSelector,
    pub ageable: Ageable,
    pub hunger: Hunger,
}

impl AnimalBundle {
    pub fn new(id: EntityId, position: Vec3, age: u32, spec: AnimalSpec) -> Self {
        let mut goals = GoalSelector::default();
        for (priority, goal) in spec.goals {
            goals.add_goal(priority, goal);
        }
        Self {
            base: EntityBaseBundle::new(id, spec.name, position),
            health: Health::new(spec.health, spec.max_health),
            move_speed: MoveSpeed(spec.move_speed),
            yaw: Yaw(0.0),
            goals,
            ageable: Ageable::new(age, spec.adult_age),
            hunger: Hunger::new(spec.hunger_limit),
        }
    }
}
