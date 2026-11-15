//! `steel-current`: minimal version of Steel's trait-based entity / goal / block-entity design.
//! Same traits and call chain as Steel; each function only does a few lines of work and logs.
//! The world is only a host (entity list + tick); there is no physics or terrain.
#![allow(dead_code)]

mod block_entity;
mod entity;
mod util;
mod world;

use block_entity::{CampfireBlockEntity, FurnaceBlockEntity};
use entity::entities::{chicken::ChickenEntity, cow::CowEntity, falling_block::FallingBlockEntity, item::ItemEntity};
use entity::{BABY_START_AGE, DamageSource, Entity, Mob};
use util::{ItemStack, Vec3};
use world::World;

fn main() {
    let world = World::new();

    let mom = world.spawn(|id| CowEntity::new(id, Vec3::new(0.0, 0.0, 0.0), 0));
    let dad = world.spawn(|id| CowEntity::new(id, Vec3::new(2.0, 0.0, 0.0), 0));
    world.spawn(|id| CowEntity::new(id, Vec3::new(-6.0, 0.0, 0.0), BABY_START_AGE));
    let hen = world.spawn(|id| ChickenEntity::new(id, Vec3::new(5.0, 0.0, 0.0), 0));
    world.spawn(|id| ItemEntity::new(id, Vec3::new(0.0, 0.0, 5.0), ItemStack::new("stone", 1)));
    world.spawn(|id| FallingBlockEntity::new(id, Vec3::new(8.0, 10.0, 0.0), "sand", 0.5));
    world.add_block_entity(CampfireBlockEntity::new((10, 0, 0), 30));
    world.add_block_entity(FurnaceBlockEntity::new((12, 0, 0), 3, 1));

    mom.mob_interact(ItemStack::new("wheat", 1));
    dad.mob_interact(ItemStack::new("wheat", 1));
    hen.hurt(DamageSource::mob_attack(mom.id()), 1.0);

    for _ in 0..200 {
        world.tick_game();
    }
    println!("--- t={} entities left: {} ---", world.game_time(), world.entities().len());
}
