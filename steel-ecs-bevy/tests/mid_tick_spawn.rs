//! "Spawned mid-tick first ticks next tick" (steel-current: tick over a snapshot).
//!
//! In bevy a system spawns via `Commands`; the entity exists only after the next sync point
//! (`ApplyDeferred`). Whether a later system in the same run sees it depends on where sync
//! points are, and bevy inserts them automatically on `chain()` / `before()` / `after()` edges.

use bevy_ecs::prelude::{
    Commands, Component, IntoScheduleConfigs, Local, Query, ResMut, Resource, Schedule, World,
};
use bevy_ecs::schedule::ApplyDeferred;

use steel_ecs_bevy::GameWorld;
use steel_ecs_bevy::entity::entities::cow;
use steel_ecs_bevy::entity::{EntityIds, tick_ageable_mobs, tick_goal_selectors};
use steel_ecs_bevy::util::Vec3;

fn spawn_cow_once(mut commands: Commands, mut ids: ResMut<EntityIds>, mut done: Local<bool>) {
    if !std::mem::replace(&mut *done, true) {
        commands.spawn(cow::bundle(ids.allocate(), Vec3::ZERO, 50));
    }
}

/// The production schedule keeps the snapshot rule, as long as the spawning system is ordered
/// into the entity pass with `*_ignore_deferred` (a plain `before/after` would add a sync point).
#[test]
fn production_schedule_spawn_in_the_middle_of_the_pass_first_ticks_next_tick() {
    let mut world = GameWorld::new();
    world.schedule_mut().add_systems(
        spawn_cow_once
            .after_ignore_deferred(tick_goal_selectors)
            .before_ignore_deferred(tick_ageable_mobs),
    );

    world.tick_game();
    let cows = world.entities();
    assert_eq!(cows.len(), 1, "spawn landed at the end-of-pass sync point");
    assert_eq!(
        world.tick_count(cows[0]),
        Some(0),
        "not ticked in the tick it was spawned"
    );

    world.tick_game();
    assert_eq!(
        world.tick_count(cows[0]),
        Some(1),
        "first tick is the next one"
    );
}

/// Same spawner, but ordered with plain `before/after`: bevy auto-inserts a sync point after
/// the spawning system, so the cow exists for the rest of the pass and is ticked immediately.
/// This is the sync-point surprise: it diverges from steel-current.
#[test]
fn plain_before_after_edges_break_the_snapshot_rule() {
    let mut world = GameWorld::new();
    world.schedule_mut().add_systems(
        spawn_cow_once
            .after(tick_goal_selectors)
            .before(tick_ageable_mobs),
    );

    world.tick_game();
    let cows = world.entities();
    assert_eq!(cows.len(), 1);
    // The auto sync point runs before the later systems, so the cow got into the remaining
    // steps of this pass (ageable/animal), but `advance_tick_count` had already run.
    // Observable difference: it is already hungry-counting in its very first tick.
    assert_eq!(
        world.tick_count(cows[0]),
        Some(0),
        "advance_tick_count ran before the spawn"
    );
    assert_eq!(
        world.hunger(cows[0]),
        Some(1),
        "but tick_animals already ran on it"
    );
}

#[derive(Component)]
struct Marker;

#[derive(Resource, Default)]
struct Seen(usize);

fn spawn_marker_once(mut commands: Commands, mut done: Local<bool>) {
    if !std::mem::replace(&mut *done, true) {
        commands.spawn(Marker);
    }
}

fn count_markers(markers: Query<&Marker>, mut seen: ResMut<Seen>) {
    seen.0 = markers.iter().count();
}

fn seen_after_first_run(schedule: &mut Schedule) -> usize {
    let mut world = World::new();
    world.init_resource::<Seen>();
    schedule.run(&mut world);
    world.resource::<Seen>().0
}

/// Minimal version of the above, independent of this crate's systems.
#[test]
fn default_chain_inserts_a_sync_point_chain_ignore_deferred_does_not() {
    let mut default_chain = Schedule::default();
    default_chain.add_systems((spawn_marker_once, count_markers).chain());
    assert_eq!(
        seen_after_first_run(&mut default_chain),
        1,
        "auto sync point: spawn visible"
    );

    let mut ignore = Schedule::default();
    ignore.add_systems((spawn_marker_once, count_markers).chain_ignore_deferred());
    assert_eq!(
        seen_after_first_run(&mut ignore),
        0,
        "no sync point: spawn not visible yet"
    );

    let mut explicit = Schedule::default();
    explicit.add_systems((spawn_marker_once, ApplyDeferred, count_markers).chain_ignore_deferred());
    assert_eq!(
        seen_after_first_run(&mut explicit),
        1,
        "explicit ApplyDeferred makes it visible"
    );
}
