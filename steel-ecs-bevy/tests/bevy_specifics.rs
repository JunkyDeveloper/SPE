//! Behaviors that exist only because the ECS is bevy_ecs. Each one is a cost or a gain the
//! other two designs do not have, pinned down so the README claims stay true.

use bevy_ecs::prelude::{Component, Query, Schedule, World};

use steel_ecs_bevy::GameWorld;
use steel_ecs_bevy::entity::Position;
use steel_ecs_bevy::util::Vec3;

#[derive(Component)]
struct A;

fn conflicting_params(_writer: Query<&mut A>, _reader: Query<&A>) {}

/// Access conflicts *inside one system* are caught when the schedule is first initialized,
/// before any entity is touched, with the offending system and component named (steel-ecs-ai:
/// panics at the second overlapping `write()` guard, mid-system; steel-current: no such check,
/// re-locking a std `Mutex` on the same thread may deadlock or panic).
///
/// It is still a runtime panic, not a compile error.
#[test]
#[should_panic(expected = "conflicts with a previous system parameter")]
fn conflicting_query_params_panic_when_the_schedule_is_initialized() {
    let mut world = World::new();
    let mut schedule = Schedule::default();
    schedule.add_systems(conflicting_params);
    schedule.run(&mut world);
}

/// The flip side of typed queries: a mob query that requires `Position` simply does not match
/// an entity without it. No panic (steel-ecs-ai: `expect("mob without Position")`), the
/// entity just silently stops taking part in that step while every other step still runs.
#[test]
fn mob_missing_a_required_component_silently_skips_the_goal_step() {
    let mut healthy = GameWorld::new();
    let cow = healthy.spawn_cow(Vec3::ZERO, 50);
    for _ in 0..45 {
        healthy.tick_game();
    }
    assert_eq!(
        healthy.hunger(cow),
        Some(1),
        "ate grass at tick 45, hunger reset then +1"
    );

    let mut broken = GameWorld::new();
    let cow = broken.spawn_cow(Vec3::ZERO, 50);
    broken.ecs_mut().entity_mut(cow).remove::<Position>();
    for _ in 0..45 {
        broken.tick_game();
    }
    assert_eq!(broken.tick_count(cow), Some(45), "still counted as ticking");
    assert_eq!(
        broken.hunger(cow),
        Some(40),
        "hungry, but the goals never ran: it never ate"
    );
}
