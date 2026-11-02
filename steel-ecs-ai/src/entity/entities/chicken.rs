//! Steel: `entity/entities/mobs/passive/chicken/mod.rs`. Full chain: Living, Mob, Ageable, Animal.

use crate::ecs::{Entity, World};
use crate::entity::{AnimalSpec, EntityId, HopGoal, LookAroundGoal, WanderGoal, spawn_animal};
use crate::util::Vec3;

/// `age` below 30 means baby.
pub fn spawn(world: &mut World, id: EntityId, position: Vec3, age: u32) -> Entity {
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
    spawn_animal(world, id, position, age, spec)
}
