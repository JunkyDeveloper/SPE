//! Steel: `entity/entities/mobs/passive/chicken/mod.rs`.

use crate::entity::{
    AnimalSpec, Entity, GoalSelector, HopGoal, Kind, LookAroundGoal, WanderGoal, pipeline,
    spawn_animal,
};
use crate::util::Vec3;

pub static CHICKEN: AnimalSpec = AnimalSpec {
    kind: Kind {
        name: "chicken",
        pipeline: pipeline::ANIMAL,
    },
    health: 2,
    max_health: 4,
    move_speed: 0.25,
    adult_age: 30,
    hunger_limit: 25,
    goals,
};

fn goals(selector: &mut GoalSelector) {
    selector.add_goal(1, HopGoal::new()); // chicken only
    selector.add_goal(5, WanderGoal::new()); // shared
    selector.add_goal(7, LookAroundGoal::new()); // shared
}

/// `age` below 30 means baby.
pub fn spawn(position: Vec3, age: u32) -> Entity {
    spawn_animal(&CHICKEN, position, age)
}
