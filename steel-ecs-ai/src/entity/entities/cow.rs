//! Steel: `entity/entities/mobs/passive/cow/mod.rs`. Full chain: Living, Mob, Ageable, Animal.
//! In ECS terms a cow is only this spawn function: a component set, no type of its own.

use crate::ecs::{Entity, World};
use crate::entity::{AnimalSpec, EatGrassGoal, EntityId, LookAroundGoal, WanderGoal, spawn_animal};
use crate::util::Vec3;

/// `age` below 50 means baby.
pub fn spawn(world: &mut World, id: EntityId, position: Vec3, age: u32) -> Entity {
    let spec = AnimalSpec {
        name: "cow",
        health: 5,
        max_health: 10,
        move_speed: 0.10,
        adult_age: 50,
        hunger_limit: 40,
        goals: vec![
            (3, Box::new(EatGrassGoal::new())),   // cow only
            (5, Box::new(WanderGoal::new())),     // shared
            (7, Box::new(LookAroundGoal::new())), // shared
        ],
    };
    spawn_animal(world, id, position, age, spec)
}
