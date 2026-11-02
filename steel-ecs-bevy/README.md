# steel-ecs-bevy

`steel-current` ported to [`bevy_ecs`](https://crates.io/crates/bevy_ecs) 0.20.1 (the standalone
ECS crate, **not** the `bevy` umbrella). Same entities (cow, chicken, item, falling block), same
goals/selector, same block entities (campfire, furnace), same tick order, same log output.
Third design next to `steel-current` (traits + mutexes) and `steel-ecs-ai` (hand-rolled ECS);
the file layout mirrors `steel-ecs-ai` so the three can be diffed side by side.
`steel-current`, `steel-ecs-ai` and `steel-ecs` are untouched.

```
cargo run -p steel-ecs-bevy     # stdout identical to `cargo run -p steel-current`
cargo test -p steel-ecs-bevy
```

```toml
bevy_ecs = { version = "0.20.1", default-features = false, features = ["std"] }
```

No `multi_threaded`, no `bevy_reflect`, no async executor. The schedule uses
`SingleThreadedExecutor` explicitly. Checked at runtime: 1 OS thread while ticking.

## Layout

| steel-current                        | steel-ecs-ai                                  | steel-ecs-bevy                                                   |
|--------------------------------------|-----------------------------------------------|------------------------------------------------------------------|
| `util.rs` (`Vec3`, `SyncMutex`)      | `util.rs` (`Vec3`)                            | `util.rs` (`Vec3`)                                               |
| `world.rs` (`World`, two managers)   | `world.rs` (registry, `Schedule` of `fn`s)    | `world.rs` (`GameWorld`, `build_schedule`, `GameTime` resource)  |
| -                                    | `ecs/` (~300 lines: entities, sparse sets, world, schedule) | **gone: `bevy_ecs` dependency**                    |
| `entity/mod.rs` (`Entity` trait)     | base components + `EntityHeader` + 3 systems | base components + `EntityBaseBundle` + 3 systems with `Query`    |
| `entity/{living,mob,ageable,animal}` | one component + one system each               | same, `#[derive(Component)]`, system params are the queries      |
| `entity/goal.rs`                     | trait objects, `&MobCtx`                      | same; `Goal: Send + Sync` (bevy components must be)              |
| `entity/entities/*`                  | `spawn(world, ...)` fns                       | `bundle(...)` fns, usable from `World::spawn` and `Commands`     |
| `block_entity.rs`                    | components + systems + `BlockEntityHeader`    | components + bundles + systems with `Query`                      |
| -                                    | `log.rs` sorted buffer                        | `log.rs` sorted buffer (`Resource`), still needed                |
| `tests.rs`                           | `tests/behavior.rs`, `golden.rs`              | same + `tests/mid_tick_spawn.rs`, `tests/bevy_specifics.rs`      |

Tick = one `Schedule` of systems: game time, tick count, move, regenerate, goals, age, hunger,
item, falling block, `ApplyDeferred`, drop removed, `ApplyDeferred`, block entities, flush logs.

## What bevy gives for free (vs `steel-ecs-ai`)

- **Typed queries.** `Query<(&EntityId, &Name, &TickCount, &mut Health), Without<Removed>>` is the
  whole parameter list of `tick_regeneration`. No hand-taken guards, no manual joins, no
  per-system `Removed` check, no `EntityHeader`/`BlockEntityHeader` helpers. `tick_goal_selectors`
  is one 9-tuple query instead of nine guards; `Option<&mut Hunger>` is the query spelling of
  steel-current's `as_animal()`.
- **Bundles.** A cow is `world.spawn(cow::bundle(id, pos, age))`; the same bundle works from
  `Commands` inside a system. No component registration step (steel-ecs-ai panics on an
  unregistered component).
- **Deferred structural changes.** `commands.entity(e).insert(Removed)` while iterating; no
  collect, drop guards, apply dance as in `steel-ecs-ai`'s `tick_items`.
- **Conflict checks with names.** Two params of one system that overlap are rejected when the
  schedule is first initialized, naming the system (`tests/bevy_specifics.rs`). Still a runtime
  panic, but before any entity is touched; steel-ecs-ai panics at the second overlapping
  `write()` guard in the middle of the system.
- **No framework to own.** ~300 lines of ECS code (+ its unit tests) replaced by a dependency.
- **Thread-safety headroom kept.** `GameWorld: Send + Sync` (compile-time test). This port still
  *ticks* on one thread; steel-ecs-ai's world is `!Send`/`!Sync` by construction.
- **A way to keep the snapshot rule** (see below), which steel-ecs-ai simply cannot express.

## What still costs (same as `steel-ecs-ai`)

- **Log buffer.** Systems run one step over all entities, so inline printing regroups lines.
  Experiment (scratch copy printing inline): exactly one line of the 44 moves (tick 40:
  `cow#0: is hungry` lands after the chicken's `health` line); same set of lines. Query
  iteration order is table order, not spawn order, so the sorted `Logs` resource stays.
- **Two ids.** `Entity` is bevy's recycled `(index, generation)` handle; the monotonic Steel id
  is its own `EntityId` component (`EntityIds` resource, so `Commands` spawns can allocate).
  Accessors on stale handles are `Option` (`world.health(cow)`), not `cow.health()`.
- **Goals are not data.** `Box<dyn Goal>` keeps private state in a component, outside the tables,
  and gets its view through the hand-built `MobCtx`. New: `Goal` needs `Send + Sync`.
- **Order is manual.** Per-entity step order = order in the schedule tuple.
- **New entity kind = more touch points:** components, bundle, system, schedule entry, accessor.

## What is new (costs bevy adds)

**Dependency weight.** `cargo tree -p steel-ecs-bevy -e normal`: **60 crates** (bevy_ecs + 59
transitive: `syn`/`quote`/`proc-macro2` stack for the derives, `toml_edit`, `hashbrown`,
`indexmap`, `bevy_tasks` + `async-task`/`crossbeam-queue`/`futures-lite`, `derive_more`,
`bevy_platform`/`bevy_ptr`/`bevy_utils`, `thread_local`, `spin`, ...). With bevy_ecs default
features it would be 82. `steel-ecs-ai` and `steel-current`: 0.

**Compile time** (fresh `--target-dir`, 16 cores, registry already downloaded):

| crate            | dev    | release |
|------------------|-------:|--------:|
| steel-ecs-bevy   | 10.9 s |  14.6 s |
| steel-ecs-ai     |  0.5 s |   0.8 s |
| steel-current    |      - |   0.4 s |

Incremental rebuild after touching `world.rs`: 0.3 s dev, 1.9 s release. The cost is the cold
build (fresh checkout, CI), not the edit loop.

**Version pinning.** bevy_ecs 0.20.1 is what cargo resolved (`rust-version = 1.95`, edition
2024). Bevy ships breaking minor versions frequently, so the schedule/executor/system
APIs used here (`Schedule::set_executor(SingleThreadedExecutor::new())`, `chain_ignore_deferred`,
`ApplyDeferred`, `before_ignore_deferred`) are a maintenance surface. Not measured; this is the
usual cost of a fast-moving dependency, stated as a risk.

**Sync-point surprise (the big one).** steel-current ticks over a snapshot, so "spawned mid-tick
first ticks next tick". In bevy a `Commands` spawn lands at the next *sync point*, and
`chain()`, `before()` and `after()` **auto-insert** `ApplyDeferred` after any system with deferred
params. Result, pinned by `tests/mid_tick_spawn.rs`:

- default `chain()`/`before()`: a cow spawned mid-pass is visible to the rest of the same pass
  and gets a partial tick in its birth tick (its `tick_animals` already ran: `hunger == 1` while
  `tick_count == 0`). **Diverges from steel-current.**
- this crate's schedule: the whole entity pass is `chain_ignore_deferred()` followed by one
  explicit `ApplyDeferred`, and spawning systems must be ordered with
  `before_ignore_deferred`/`after_ignore_deferred`. Then spawns land after the last ticking
  system and first tick next tick, like steel-current. **Preserved, but only by discipline**:
  one plain `.before(...)` on a spawner silently breaks it.

**Silent partial ticking.** A mob query that needs `Position` just does not match an entity
without it: no panic (steel-ecs-ai: `expect("mob without Position")`), the entity keeps
advancing its other steps (`tests/bevy_specifics.rs`: a cow without `Position` is "hungry but
never eats"). The type-level guarantee of `Mob: LivingEntity: Entity` is gone, same as in
steel-ecs-ai, but the failure mode is quieter.

**Smaller things.**
- `Mut<T>`: bevy keeps change ticks for every component and wraps writes for change detection.
  Unused here; not measured separately.
- clippy `type_complexity` fires on any non-trivial `Query<(...)>`: `#[allow]` or type aliases.
- `World::query` needs `&mut World`, so `GameWorld::entities(&self)` scans `iter_entities()`
  (O(all entities)) and filters on `EntityId`.
- Learning curve: Component, Bundle, Resource, Query + filters, Commands, Schedule, system
  ordering, sync points, `Mut`. steel-ecs-ai needed none of the query filters, Commands, sync points or `Mut`.
- Multi-threading: enabling `multi_threaded` and switching the executor would let disjoint
  systems run in parallel, but the entity pass is one strict chain, so there is little to
  parallelize without redesigning the order. Not tried.

## Measured

Scratch benchmark outside the repo (`.../scratchpad/bench`, not committed). Workload: N mixed
entities (`i % 4`: cow adult, chicken adult, item with lifetime `u32::MAX` and velocity 0.01,
falling block from y = 1e12) plus one campfire and one furnace (fuel 1 000 000) per 10th `i`;
200 ticks; release; stdout to `/dev/null`; 3 runs each, median shown. Single thread everywhere.
All four emit the same 178 500 lines at N = 10 000 and are byte-identical on a 2 000 x 150
variant. `current, no locks` = a copy of steel-current with `SyncMutex` swapped for an
unsynchronized cell: an ablation to attribute cost, unsound under threads, not a candidate.

| N entities | current | current, no locks | steel-ecs-ai | steel-ecs-bevy |
|-----------:|--------:|------------------:|-------------:|---------------:|
|     10 000 |  293 ms |            117 ms |       93 ms  |         89 ms  |
|    100 000 |  3.21 s |            1.41 s |      1.13 s  |        0.98 s  |

Run-to-run spread within each cell was a few percent (raw runs: bevy 88-90 ms and
0.95-1.04 s; ecs 92-95 ms and 1.00-1.13 s). This is a fresh run, so figures differ a little from
`steel-ecs-ai/README.md`.

Bevy is about as fast as the hand-rolled ECS here, slightly ahead (4 % at 10k, ~14 % at 100k),
despite change-detection bookkeeping. Plausible reason (not profiled; `perf` is blocked here):
bevy's tables co-locate a query's components, while the raw crate does one sparse-set lookup per
component per entity for joins. Caveats: synthetic workload, uncontended locks in `current`,
one machine, no profile.

## Size

Code lines (comments, blanks and inline tests excluded): steel-current ~880; steel-ecs-bevy
~995 of game code and no ECS code; steel-ecs-ai ~1030 of game code plus ~300 of ECS. Both
ECS ports were run through rustfmt, which expands some lines, so they read slightly larger than
steel-current's compact style.
