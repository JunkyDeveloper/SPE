//! Port of `steel-current/src/tests.rs` (same scenarios, same numbers), plus extras for removed
//! entities and ids.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use steel_ecs_ai::GameWorld;
use steel_ecs_ai::entity::{
    Cx, Entity, EntityId, FLAG_MOVE, Goal, GoalSelector, Kind, Mob, pipeline,
};
use steel_ecs_ai::util::Vec3;

fn run(world: &mut GameWorld, ticks: usize) {
    for _ in 0..ticks {
        world.tick_game();
    }
}

#[test]
fn falling_block_is_linear_then_lands() {
    let mut world = GameWorld::new();
    let b = world.spawn_falling_block(0.0, 10.0, 0.0, 0.5);
    run(&mut world, 4);
    assert_eq!(world.position(b).unwrap().y, 8.0);
    run(&mut world, 16);
    assert!(world.is_removed(b));
    assert!(world.entities().is_empty());
}

#[test]
fn item_moves_linearly_and_despawns() {
    let mut world = GameWorld::new();
    let i = world.spawn_item(Vec3::ZERO, Vec3::new(0.5, 0.0, 0.0), 10);
    run(&mut world, 5);
    assert_eq!(world.position(i).unwrap().x, 2.5);
    run(&mut world, 5);
    assert!(world.is_removed(i));
}

#[test]
fn baby_grows_up_and_regenerates() {
    let mut world = GameWorld::new();
    let cow = world.spawn_cow(Vec3::ZERO, 0);
    assert_eq!(world.is_baby(cow), Some(true));
    run(&mut world, 50);
    assert_eq!(world.is_baby(cow), Some(false));
    assert_eq!(world.health(cow), Some(7));
    run(&mut world, 50);
    assert_eq!(world.health(cow), Some(10), "capped at max");
}

#[test]
fn wander_moves_x_for_ten_ticks_then_stops() {
    let mut world = GameWorld::new();
    let cow = world.spawn_cow(Vec3::ZERO, 50);
    run(&mut world, 29);
    assert_eq!(world.position(cow).unwrap().x, 0.0);
    run(&mut world, 11); // tick 40
    assert!(
        (world.position(cow).unwrap().x - 1.0).abs() < 1e-9,
        "10 ticks * 0.1"
    );
    run(&mut world, 1);
    assert!(
        (world.position(cow).unwrap().x - 1.0).abs() < 1e-9,
        "stopped"
    );
}

#[test]
fn cow_eats_grass_when_hungry() {
    let mut world = GameWorld::new();
    let cow = world.spawn_cow(Vec3::ZERO, 50);
    run(&mut world, 40);
    assert_eq!(world.is_hungry(cow), Some(true));
    run(&mut world, 3); // eating
    assert_eq!(world.velocity(cow).unwrap().x, 0.0);
    assert_eq!(world.hunger(cow), Some(40));
    run(&mut world, 2); // tick 45: goal resets hunger, animal tick adds 1
    assert_eq!(world.hunger(cow), Some(1));
}

#[test]
fn chicken_has_no_eat_goal_and_hops() {
    let mut world = GameWorld::new();
    let hen = world.spawn_chicken(Vec3::ZERO, 30);
    run(&mut world, 100);
    assert_eq!(world.is_hungry(hen), Some(true));
    assert_eq!(world.hunger(hen), Some(25), "capped, never reset");
    run(&mut world, 20); // tick 120: hop starts, y +0.25
    assert_eq!(world.position(hen).unwrap().y, 0.25);
    run(&mut world, 3); // up to 0.5, back down to 0.0
    assert_eq!(world.position(hen).unwrap().y, 0.0);
}

#[test]
fn look_around_turns_head() {
    let mut world = GameWorld::new();
    let cow = world.spawn_cow(Vec3::ZERO, 50);
    run(&mut world, 20); // look at tick 15..19
    assert_eq!(world.yaw(cow), Some(45.0));
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
        fn can_use(&mut self, _e: &Entity, _cx: &Cx) -> bool {
            self.want.load(Ordering::Relaxed)
        }
        fn start(&mut self, _e: &mut Entity, _cx: &mut Cx) {
            self.log
                .lock()
                .unwrap()
                .push(format!("start {}", self.name));
        }
        fn stop(&mut self, _e: &mut Entity, _cx: &mut Cx) {
            self.log.lock().unwrap().push(format!("stop {}", self.name));
        }
    }

    // steel-current borrowed a spawned cow as `&dyn Mob`; a goal's view of a mob is just an
    // `Entity` with a `Mob` part, so the test builds a bare one, no spawning needed.
    static PROBE: Kind = Kind {
        name: "probe",
        pipeline: pipeline::ANIMAL,
    };
    let mut world = GameWorld::new();
    let mut mob = Entity::new(&PROBE, Vec3::ZERO);
    mob.mob = Some(Mob::new(0.1));

    let log = Arc::new(Mutex::new(Vec::new()));
    let (low_want, high_want) = (
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(false)),
    );
    let mut sel = GoalSelector::default();
    sel.add_goal(
        5,
        Probe {
            name: "low",
            want: low_want,
            log: log.clone(),
        },
    );
    sel.add_goal(
        1,
        Probe {
            name: "high",
            want: high_want.clone(),
            log: log.clone(),
        },
    );
    sel.tick(&mut mob, &mut world.cx());
    high_want.store(true, Ordering::Relaxed);
    sel.tick(&mut mob, &mut world.cx());
    sel.tick(&mut mob, &mut world.cx());
    assert_eq!(
        *log.lock().unwrap(),
        ["start low", "stop low", "start high"]
    );
}

#[test]
fn block_entities_do_counter_math() {
    let mut world = GameWorld::new();
    let fire = world.add_campfire((0, 0, 0), 30);
    let furnace = world.add_furnace((1, 0, 0), 25);
    run(&mut world, 100);
    assert_eq!(world.campfire_cooked(fire), Some(3));
    assert_eq!(world.furnace_output(furnace), Some(2));
    assert_eq!(world.furnace_fuel(furnace), Some(0));
}

/// steel-current: `tick_entities` skips `is_removed()` entities (no tick, no tick count) and
/// drops them after the pass. Not covered by its tests, but part of the behavior.
#[test]
fn externally_removed_entity_is_skipped_then_dropped() {
    let mut world = GameWorld::new();
    let cow = world.spawn_cow(Vec3::ZERO, 50);
    let hen = world.spawn_chicken(Vec3::ZERO, 30);
    run(&mut world, 3);
    world.set_removed(cow);
    assert!(world.is_removed(cow));
    assert_eq!(
        world.entities().len(),
        2,
        "still listed until the next pass drops it"
    );
    assert_eq!(world.tick_count(cow), Some(3));

    run(&mut world, 1);
    assert_eq!(world.tick_count(cow), None, "dropped, not ticked");
    assert_eq!(world.entities(), vec![hen]);
    assert_eq!(world.tick_count(hen), Some(4));
}

#[test]
fn entity_ids_are_monotonic_and_never_reused() {
    let mut world = GameWorld::new();
    let block = world.spawn_falling_block(0.0, 0.5, 0.0, 0.5); // lands on tick 1
    run(&mut world, 1);
    assert!(world.is_removed(block));
    let cow = world.spawn_cow(Vec3::ZERO, 50);
    assert_eq!(world.id(cow), Some(EntityId(1)), "id 0 is not recycled");
    assert_ne!(block, cow, "recycled slot has a new generation");
}
