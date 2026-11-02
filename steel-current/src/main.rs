//! `steel-current`: minimal version of Steel's trait-based entity / block-entity / tick design.
//! Entities only manipulate small bits of data and print log lines.
#![allow(dead_code)]

mod block_entity;
mod entity;
mod util;
mod world;

#[cfg(test)]
mod tests;

use block_entity::{CampfireBlockEntity, FurnaceBlockEntity};
use entity::entities::{chicken::ChickenEntity, cow::CowEntity, falling_block::FallingBlockEntity, item::ItemEntity};
use util::Vec3;
use world::World;

fn main() {
    let world = World::new();

    world.spawn(|id| CowEntity::new(id, Vec3::new(0.0, 0.0, 0.0), 45));
    world.spawn(|id| ChickenEntity::new(id, Vec3::new(5.0, 0.0, 0.0), 30));
    world.spawn(|id| ItemEntity::new(id, Vec3::new(0.0, 1.0, 5.0), Vec3::new(0.1, 0.0, 0.0), 40));
    world.spawn(|id| FallingBlockEntity::new(id, 8.0, 10.0, 0.0, 0.5));
    world.add_block_entity(CampfireBlockEntity::new((10, 0, 0), 30));
    world.add_block_entity(FurnaceBlockEntity::new((12, 0, 0), 35));

    for _ in 0..100 {
        world.tick_game();
    }
    println!("--- t={} entities left: {} ---", world.game_time(), world.entities.entities().len());
}
