//! What the design is *for*, tested from outside the crate (so only the public API is used):
//!
//! - a new mob is a value, not a type hierarchy;
//! - a custom tick (own stage list) can be written without touching the crate;
//! - entities tick one at a time, so an entity's effect on another is visible to it in the same
//!   tick (the property a "run each step over all entities" design loses);
//! - things spawned mid-tick first tick next tick;
//! - the world can move to another thread.

use steel_ecs_ai::GameWorld;
use steel_ecs_ai::entity::{
    AnimalSpec, Cx, Entity, EntityId, GoalSelector, Kind, WanderGoal, apply_velocity, pipeline,
    spawn_animal,
};
use steel_ecs_ai::util::Vec3;

fn run(world: &mut GameWorld, ticks: usize) {
    for _ in 0..ticks {
        world.tick_game();
    }
}

// ---- a new mob: this is all it takes ----

static PIG: AnimalSpec = AnimalSpec {
    kind: Kind {
        name: "pig",
        pipeline: pipeline::ANIMAL,
    },
    health: 5,
    max_health: 10,
    move_speed: 0.2,
    adult_age: 20,
    hunger_limit: 15,
    goals: pig_goals,
};

fn pig_goals(selector: &mut GoalSelector) {
    selector.add_goal(5, WanderGoal::new());
}

#[test]
fn a_new_mob_is_just_a_spec() {
    let mut world = GameWorld::new();
    let pig = world.spawn(spawn_animal(&PIG, Vec3::ZERO, 0));
    assert_eq!(world.is_baby(pig), Some(true));
    run(&mut world, 20);
    assert_eq!(
        world.is_baby(pig),
        Some(false),
        "grew up at its own adult age"
    );
    run(&mut world, 11); // tick 31: wander started at 30, first step east at 31
    assert_eq!(world.is_hungry(pig), Some(true), "hunger limit 15");
    assert!(
        (world.position(pig).unwrap().x - 0.2).abs() < 1e-9,
        "its own speed"
    );
}

// ---- custom stages + cross-entity effects ----

/// Pushes every other entity closer than one block away along x, one-sided and immediately.
fn push_neighbours(e: &mut Entity, cx: &mut Cx) {
    let me = e.base.position;
    cx.for_each_other(|other| {
        let dx = other.base.position.x - me.x;
        if dx.abs() < 1.0 {
            other.base.velocity.x += 0.1 * dx.signum();
        }
    });
}

static PUSHER: Kind = Kind {
    name: "pusher",
    pipeline: &[apply_velocity, push_neighbours],
};

#[test]
fn an_entitys_effect_on_a_later_entity_is_visible_in_the_same_tick() {
    let mut world = GameWorld::new();
    let a = world.spawn(Entity::new(&PUSHER, Vec3::new(0.0, 0.0, 0.0)));
    let b = world.spawn(Entity::new(&PUSHER, Vec3::new(0.5, 0.0, 0.0)));

    world.tick_game();

    // a ticks first: it moves (velocity 0), then pushes b. b then ticks with the new velocity
    // and moves in this very tick. Running "all moves, then all pushes" would leave b at 0.5.
    assert_eq!(world.position(a).unwrap().x, 0.0);
    assert!((world.position(b).unwrap().x - 0.6).abs() < 1e-9);

    // b pushed a back (b was already at 0.6, a is at -0.6 away): a only moves next tick.
    assert!((world.velocity(a).unwrap().x + 0.1).abs() < 1e-9);
    world.tick_game();
    assert!((world.position(a).unwrap().x + 0.1).abs() < 1e-9);
}

#[test]
fn an_entity_never_sees_itself_through_the_context() {
    fn count_others(e: &mut Entity, cx: &mut Cx) {
        let mut seen = 0.0;
        cx.for_each_other(|_| seen += 1.0);
        e.base.position.y = seen;
    }
    static COUNTER: Kind = Kind {
        name: "counter",
        pipeline: &[count_others],
    };
    let mut world = GameWorld::new();
    let a = world.spawn(Entity::new(&COUNTER, Vec3::ZERO));
    world.spawn(Entity::new(&COUNTER, Vec3::ZERO));
    world.spawn(Entity::new(&COUNTER, Vec3::ZERO));
    world.tick_game();
    assert_eq!(
        world.position(a).unwrap().y,
        2.0,
        "the other two, not itself"
    );
}

// ---- spawning during a tick ----

#[test]
fn spawned_mid_tick_first_ticks_next_tick() {
    fn spawn_child_once(e: &mut Entity, cx: &mut Cx) {
        if e.base.tick_count == 1 {
            cx.spawn(Entity::new(&CHILD, e.base.position));
        }
    }
    static PARENT: Kind = Kind {
        name: "parent",
        pipeline: &[spawn_child_once],
    };
    static CHILD: Kind = Kind {
        name: "child",
        pipeline: &[apply_velocity],
    };

    let mut world = GameWorld::new();
    world.spawn(Entity::new(&PARENT, Vec3::ZERO));

    world.tick_game();
    let entities = world.entities();
    assert_eq!(entities.len(), 2, "child exists right away");
    let child = entities[1];
    assert_eq!(
        world.tick_count(child),
        Some(0),
        "but has not ticked in its birth tick"
    );
    assert_eq!(world.id(child), Some(EntityId(1)));

    world.tick_game();
    assert_eq!(world.tick_count(child), Some(1));
}

// ---- threads ----

#[test]
fn the_world_can_move_to_another_thread() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<GameWorld>();

    let mut world = GameWorld::new();
    world.spawn_cow(Vec3::ZERO, 50);
    let world = std::thread::spawn(move || {
        run(&mut world, 10);
        world
    })
    .join()
    .unwrap();
    assert_eq!(world.entities().len(), 1);
}
