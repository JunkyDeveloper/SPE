//! Same scenario as `steel-current`'s `main`; stdout is expected to match it byte for byte.

use steel_ecs_ai::GameWorld;
use steel_ecs_ai::util::Vec3;

fn main() {
    let mut world = GameWorld::new();

    world.spawn_cow(Vec3::new(0.0, 0.0, 0.0), 45);
    world.spawn_chicken(Vec3::new(5.0, 0.0, 0.0), 30);
    world.spawn_item(Vec3::new(0.0, 1.0, 5.0), Vec3::new(0.1, 0.0, 0.0), 40);
    world.spawn_falling_block(8.0, 10.0, 0.0, 0.5);
    world.add_campfire((10, 0, 0), 30);
    world.add_furnace((12, 0, 0), 35);

    for _ in 0..100 {
        world.tick_game();
    }
    println!(
        "--- t={} entities left: {} ---",
        world.game_time(),
        world.entities().len()
    );
}
