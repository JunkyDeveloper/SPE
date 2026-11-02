//! Steel: `entity/entities/mobs/passive/cow/mod.rs`. 85 lines and five trait impls in
//! steel-current; here it is data.

use crate::entity::{
    AnimalSpec, EatGrassGoal, Entity, GoalSelector, Kind, LookAroundGoal, WanderGoal, pipeline,
    spawn_animal,
};
use crate::util::Vec3;

pub static COW: AnimalSpec = AnimalSpec {
    kind: Kind {
        name: "cow",
        pipeline: pipeline::ANIMAL,
    },
    health: 5,
    max_health: 10,
    move_speed: 0.10,
    adult_age: 50,
    hunger_limit: 40,
    goals,
};

fn goals(selector: &mut GoalSelector) {
    selector.add_goal(3, EatGrassGoal::new()); // cow only
    selector.add_goal(5, WanderGoal::new()); // shared
    selector.add_goal(7, LookAroundGoal::new()); // shared
}

/// `age` below 50 means baby.
pub fn spawn(position: Vec3, age: u32) -> Entity {
    spawn_animal(&COW, position, age)
}
