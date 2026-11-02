use crate::block_entity::{CampfireBlockEntity, FurnaceBlockEntity};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::entity::entities::{chicken::ChickenEntity, cow::CowEntity, falling_block::FallingBlockEntity, item::ItemEntity};
use crate::entity::*;
use crate::util::Vec3;
use crate::world::World;

fn run(world: &World, ticks: usize) {
    for _ in 0..ticks {
        world.tick_game();
    }
}

#[test]
fn falling_block_is_linear_then_lands() {
    let world = World::new();
    let b = world.spawn(|id| FallingBlockEntity::new(id, 0.0, 10.0, 0.0, 0.5));
    run(&world, 4);
    assert_eq!(b.position().y, 8.0);
    run(&world, 16);
    assert!(b.is_removed());
    assert!(world.entities.entities().is_empty());
}

#[test]
fn item_moves_linearly_and_despawns() {
    let world = World::new();
    let i = world.spawn(|id| ItemEntity::new(id, Vec3::ZERO, Vec3::new(0.5, 0.0, 0.0), 10));
    run(&world, 5);
    assert_eq!(i.position().x, 2.5);
    run(&world, 5);
    assert!(i.is_removed());
}

#[test]
fn baby_grows_up_and_regenerates() {
    let world = World::new();
    let cow = world.spawn(|id| CowEntity::new(id, Vec3::ZERO, 0));
    assert!(cow.is_baby());
    run(&world, 50);
    assert!(!cow.is_baby());
    assert_eq!(cow.health(), 7);
    run(&world, 50);
    assert_eq!(cow.health(), 10, "capped at max");
}

#[test]
fn wander_moves_x_for_ten_ticks_then_stops() {
    let world = World::new();
    let cow = world.spawn(|id| CowEntity::new(id, Vec3::ZERO, 50));
    run(&world, 29);
    assert_eq!(cow.position().x, 0.0);
    run(&world, 11); // tick 40
    assert!((cow.position().x - 1.0).abs() < 1e-9, "10 ticks * 0.1");
    run(&world, 1);
    assert!((cow.position().x - 1.0).abs() < 1e-9, "stopped");
}

#[test]
fn cow_eats_grass_when_hungry() {
    let world = World::new();
    let cow = world.spawn(|id| CowEntity::new(id, Vec3::ZERO, 50));
    run(&world, 40);
    assert!(cow.is_hungry());
    run(&world, 3); // eating
    assert_eq!(cow.velocity().x, 0.0);
    assert_eq!(cow.hunger(), 40);
    run(&world, 2); // tick 45: goal resets hunger, animal tick adds 1
    assert_eq!(cow.hunger(), 1);
}

#[test]
fn chicken_has_no_eat_goal_and_hops() {
    let world = World::new();
    let hen = world.spawn(|id| ChickenEntity::new(id, Vec3::ZERO, 30));
    run(&world, 100);
    assert!(hen.is_hungry());
    assert_eq!(hen.hunger(), 25, "capped, never reset");
    run(&world, 20); // tick 120: hop starts, y +0.25
    assert_eq!(hen.position().y, 0.25);
    run(&world, 3); // up to 0.5, back down to 0.0
    assert_eq!(hen.position().y, 0.0);
}

#[test]
fn look_around_turns_head() {
    let world = World::new();
    let cow = world.spawn(|id| CowEntity::new(id, Vec3::ZERO, 50));
    run(&world, 20); // look at tick 15..19
    assert_eq!(cow.yaw(), 45.0);
}

#[test]
fn selector_preempts_less_important_conflicting_goal() {
    struct Probe {
        name: &'static str,
        want: Arc<AtomicBool>,
        log: Arc<Mutex<Vec<String>>>,
    }
    impl Goal for Probe {
        fn flags(&self) -> u8 {
            FLAG_MOVE
        }
        fn can_use(&mut self, _m: &dyn Mob) -> bool {
            self.want.load(Ordering::Relaxed)
        }
        fn start(&mut self, _m: &dyn Mob) {
            self.log.lock().unwrap().push(format!("start {}", self.name));
        }
        fn stop(&mut self, _m: &dyn Mob) {
            self.log.lock().unwrap().push(format!("stop {}", self.name));
        }
    }

    let world = World::new();
    let cow = world.spawn(|id| CowEntity::new(id, Vec3::ZERO, 50));
    let log = Arc::new(Mutex::new(Vec::new()));
    let (low_want, high_want) = (Arc::new(AtomicBool::new(true)), Arc::new(AtomicBool::new(false)));
    let mut sel = GoalSelector::default();
    sel.add_goal(5, Box::new(Probe { name: "low", want: low_want, log: log.clone() }));
    sel.add_goal(1, Box::new(Probe { name: "high", want: high_want.clone(), log: log.clone() }));
    let mob: &dyn Mob = &*cow;
    sel.tick(mob);
    high_want.store(true, Ordering::Relaxed);
    sel.tick(mob);
    sel.tick(mob);
    assert_eq!(*log.lock().unwrap(), ["start low", "stop low", "start high"]);
}

#[test]
fn block_entities_do_counter_math() {
    let world = World::new();
    let fire = world.add_block_entity(CampfireBlockEntity::new((0, 0, 0), 30));
    let furnace = world.add_block_entity(FurnaceBlockEntity::new((1, 0, 0), 25));
    run(&world, 100);
    assert_eq!(fire.cooked(), 3);
    assert_eq!(furnace.output(), 2);
    assert_eq!(furnace.fuel(), 0);
}
