//! Steel: `entity/entities/mobs/passive/chicken/mod.rs`. Full chain: Living, Mob, Ageable, Animal.

use crate::entity::{AnimalBundle, AnimalSpec, EntityId, HopGoal, LookAroundGoal, WanderGoal};
use crate::util::Vec3;

/// `age` below 30 means baby.
pub fn bundle(id: EntityId, position: Vec3, age: u32) -> AnimalBundle {
    let spec = AnimalSpec {
        name: "chicken",
        health: 2,
        max_health: 4,
        move_speed: 0.25,
        adult_age: 30,
        hunger_limit: 25,
        goals: vec![
            (1, Box::new(HopGoal::new())),        // chicken only
            (5, Box::new(WanderGoal::new())),     // shared
            (7, Box::new(LookAroundGoal::new())), // shared
        ],
    };
    AnimalBundle::new(id, position, age, spec)
}
