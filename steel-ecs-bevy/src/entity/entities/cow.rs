//! Steel: `entity/entities/mobs/passive/cow/mod.rs`. Full chain: Living, Mob, Ageable, Animal.
//! In ECS terms a cow is only this bundle constructor: a component set, no type of its own.

use crate::entity::{AnimalBundle, AnimalSpec, EatGrassGoal, EntityId, LookAroundGoal, WanderGoal};
use crate::util::Vec3;

/// `age` below 50 means baby.
pub fn bundle(id: EntityId, position: Vec3, age: u32) -> AnimalBundle {
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
    AnimalBundle::new(id, position, age, spec)
}
