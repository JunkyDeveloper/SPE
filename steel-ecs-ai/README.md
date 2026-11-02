# steel-ecs-ai

`steel-current` redesigned **without a trait hierarchy**: same entities (cow, chicken, item,
falling block), same goals/selector, same block entities (campfire, furnace), same tick order,
same log output. `steel-current` and `steel-ecs` are untouched.

The crate name is left over from the first attempt here (a hand-written ECS, replaced by this).
There is no ECS in the crate any more.

```
cargo run -p steel-ecs-ai     # stdout identical to `cargo run -p steel-current`
cargo test -p steel-ecs-ai
```

## The idea

Two things Java's inheritance bundles together are separated:

- **What a mob is** = the parts it carries. `Entity` is one struct: `Base` (id, position,
  velocity, tick count) plus `Option<Living>`, `Option<Mob>`, `Option<Ageable>`,
  `Option<Animal>`, and object parts (`Item`, `FallingBlock`). Public fields, no accessors to
  override.
- **What a tick does** = its `Kind`'s pipeline: a `&[Stage]` of plain
  `fn(&mut Entity, &mut Cx)`, run in order. The vanilla step order is written **once** in
  `entity/pipeline.rs` instead of being re-chained by hand in every concrete type.

```rust
// entity/pipeline.rs
pub const ANIMAL: &[Stage] = &[
    apply_velocity,       // Entity::base_tick
    living::regenerate,   // LivingEntity
    mob::run_goals,       // Mob
    ageable::age_up,      // AgeableMob
    animal::get_hungrier, // Animal
];
```

A mob is a value (`entity/entities/cow.rs`, 85 lines + 5 trait impls in steel-current):

```rust
pub static COW: AnimalSpec = AnimalSpec {
    kind: Kind { name: "cow", pipeline: pipeline::ANIMAL },
    health: 5, max_health: 10, move_speed: 0.10, adult_age: 50, hunger_limit: 40,
    goals,
};
fn goals(s: &mut GoalSelector) {
    s.add_goal(3, EatGrassGoal::new());
    s.add_goal(5, WanderGoal::new());
    s.add_goal(7, LookAroundGoal::new());
}
```

`tests/composition.rs` adds a pig this way from *outside* the crate, and defines custom stages
(a push between entities) without touching any crate code.

## What did NOT change: the tick

Ticking is still **entity by entity** (`GameWorld::tick_entities` = the old
`EntityManager::tick_entities`): spawn order, one entity through *all* its stages before the next.
So:

- vanilla order is kept, including an entity changing a later entity that then ticks with the
  change in the *same* tick (`push_entity` in Steel). A design that runs each step over all
  entities cannot do that. Test: `an_entitys_effect_on_a_later_entity_is_visible_in_the_same_tick`.
- "spawned mid-tick first ticks next tick" is kept: the loop runs over the entities that existed
  at tick start; `Cx::spawn` appends behind that bound. Test: `spawned_mid_tick_first_ticks_next_tick`.
- removed entities are skipped and dropped after the pass.
- log lines are printed inline. The previous raw-ECS version needed a sorted log buffer to fake
  this order; here stdout is byte-identical with no such machinery (`tests/golden.rs`).

## Layout

| steel-current                              | steel-ecs-ai                                              |
|--------------------------------------------|-----------------------------------------------------------|
| `entity/mod.rs` `Entity` trait + `EntityBase` | `entity/mod.rs` `Entity` struct, `Base`, `Kind`, `Stage`  |
| `living.rs` / `mob.rs` / `ageable.rs` / `animal.rs` (trait + `*Base`) | same files: one part + one stage each |
| `ai_step` chained by hand per concrete type | `entity/pipeline.rs`, once                                |
| `entities/{cow,chicken}.rs` (5 trait impls) | `AnimalSpec` values + `spawn_animal`                       |
| `entities/{item,falling_block}.rs`         | part + stage + spawn fn each                              |
| `entity/goal.rs` (`&dyn Mob`)              | `entity/goal.rs` (`&mut Entity` + `&mut Cx`)              |
| `world.rs` two managers, `Arc` + `Mutex`   | `entity/store.rs` (`Store`, `Cx`) + `world.rs`            |
| `block_entity.rs` (trait)                  | `block_entity.rs` (same pattern, smaller)                 |
| `tests.rs`                                 | `tests/behavior.rs` (9 ported + 2), `tests/composition.rs`, `tests/golden.rs` |

## What it costs (and what is not solved)

- **Nothing can touch an entity from another thread.** steel-current is `Arc<dyn Entity>` with a
  mutex per field, built so packet workers, the chunk sender and other world threads *can*
  reach entities (nothing here does that while the tick runs, but the types allow it). Here the
  world owns its entities and ticks through `&mut`: `GameWorld` is `Send + Sync` (test: moves to
  another thread), but mutating it from several threads means a lock around the whole world or a
  command queue. This is the main design decision, not a detail. Steel's players, trackers and
  `Weak` references would have to be redesigned around it; none of that is modeled here.
- **Handles instead of `Arc`.** `Handle` is generation-checked, so every access is an `Option`
  (`world.health(cow)`, not `cow.health()`). Held handles go stale on despawn instead of keeping
  the entity alive. There are still two ids: `Handle` (recycled slot) and `EntityId` (Steel-style
  network id, monotonic, never reused).
- **A ticking entity is invisible to itself through `Cx`.** To hold `&mut Entity` and `&mut Cx` at
  once, the entity is moved out of its slot while it ticks (`Store::take`). `cx.get(own_handle)`
  is `None`, `for_each_other` skips it (what pushing wants anyway). Same trick for the goal
  selector inside `Mob` (`entity.mob().goals` is empty while goals run).
- **"Entity has this part" is checked at runtime.** A stage on an entity without its part panics
  with a message naming the part (`entity.living()` etc.). steel-current guaranteed it with
  `Mob: LivingEntity: Entity`. A fix would be a check at spawn that the kind's pipeline is
  satisfied by the parts; not done here.
- **Every entity has a slot for every part.** `size_of::<Entity>() == 184` bytes, boxed. Rare
  parts could be boxed; irrelevant at this size.
- **Goals are still trait objects** with private state; they now get the whole `&mut Entity`
  instead of a trait surface, which is the part that got easier to write.
- **`Cx::for_each_other` is a linear scan.** It is where Steel's spatial index would plug in.
- **Only the shared tick order is modeled.** There is no push, targeting or riding in the main
  scenario (steel-current has none); `tests/composition.rs` has a minimal stand-in for the push.
  Steel's real entity code (vehicle chains, relationships, callbacks) is not touched by this
  prototype.

## Measured

Release, median of 3, stdout to /dev/null. Scratch benchmark outside the repo: N mixed entities
(cow/chicken/item/falling block) + N/5 block entities, 200 ticks, one thread everywhere. All
variants print the same output (checked by hash on a 2000-entity run; 178 500 lines at N = 10 000).
`current, no lock` = a copy of `steel-current` with the mutex replaced by an unsynchronized cell
(ablation only, unsound under threads). `raw ECS` = the previous version of this crate.

| N entities | current | current, no lock | raw ECS | bevy_ecs | **this crate** |
|-----------:|--------:|-----------------:|--------:|---------:|---------------:|
|     10 000 |  302 ms |           112 ms |   94 ms |    91 ms |      **108 ms** |
|    100 000 |  3.36 s |           1.49 s |  1.06 s |   1.00 s |      **1.38 s** |

Reading it:
- Against steel-current: 2.8x (10k) and 2.4x (100k) faster. Most of that is the lock and the
  `Arc` traffic, which entities no longer pay (steel-current is 302 -> 112 ms by removing the lock
  alone).
- Against the column-based ECS: 15-30% slower. The columnar versions read packed arrays; this
  one reads a 184-byte boxed struct per entity. That is my explanation, not a measurement:
  `perf` is blocked on this machine.
- This is a toy workload (no collisions, no world queries, no pathfinding). Real entity ticking in
  Steel spends its time in exactly those, which none of these numbers say anything about.
