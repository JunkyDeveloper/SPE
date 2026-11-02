# steel-ecs-ai

`steel-current` ported to a hand-written, std-only ECS. Same entities (cow, chicken, item,
falling block), same goals/selector, same block entities (campfire, furnace), same tick order,
same log output. `steel-current` is untouched; `steel-ecs` is untouched (empty, yours).

```
cargo run -p steel-ecs-ai     # stdout identical to `cargo run -p steel-current`
cargo test -p steel-ecs-ai
```

## Layout

| steel-current                         | steel-ecs-ai                                                    |
|---------------------------------------|-----------------------------------------------------------------|
| `util.rs` (`Vec3`, `SyncMutex`)       | `util.rs` (`Vec3` only, no locks)                               |
| `world.rs` (`World`, two managers)    | `world.rs` (`GameWorld`, component registry, schedule)          |
| `entity/mod.rs` (`Entity` trait)      | `entity/mod.rs` (base components + shared systems)              |
| `entity/{living,mob,ageable,animal}`  | same files: one component + one system each                     |
| `entity/goal.rs`                      | `entity/goal.rs` (still trait objects, `&MobCtx` not `&dyn Mob`) |
| `entity/entities/*`                   | same files: a spawn function (+ component/system for item, block) |
| `block_entity.rs`                     | `block_entity.rs` (components + systems)                        |
| `tests.rs`                            | `tests/behavior.rs` (same 9 scenarios + 2 extra), `tests/golden.rs` |
| -                                     | `ecs/` the framework: entities, sparse sets, world, schedule    |
| -                                     | `log.rs` deferred, sorted log buffer                            |

Tick = one schedule of plain `fn(&mut World)` systems, in the order `Entity::tick` used to chain
by hand: tick count, move, regenerate, goals, age, hunger, item, falling block, drop removed,
block entities, flush logs.

## What the ECS port costs

Everything below was hit while porting, not guessed.

**Framework you now own (~300 lines, not counting its tests).** Generational entity ids, one
`SparseSet` per component type, type-erased storages so despawn can strip an entity from all of
them, resources, a schedule. A library would supply this, but it is the part that has to be
understood and debugged.

**No typed queries.** Every system manually takes guards (`world.read::<T>()` /
`world.write::<T>()`) and joins by hand: iterate one storage, `get(entity)` in the others,
re-check `Removed` in every system. `EntityHeader` / `BlockEntityHeader` exist only to cut that
boilerplate. `MobCtx` is the same idea for goals.

**Runtime borrow checking.** Storages sit in `RefCell`s so a system can hold several at once
through `&World`. Two `write::<T>()` of the same type is a *runtime* panic (see the test).
Nothing declares what a system reads/writes, so nothing can schedule systems in parallel, and
`World` is `!Sync`. Execution is the same as steel-current today: both tick on one thread
(`tick_game` is sequential loops; no thread is ever spawned, checked at runtime: 1 OS thread).
The difference is headroom, not current behavior: steel-current is *built* thread-safe
(`&self`, `Send + Sync`, mutex per field, atomics, `Arc`) so a caller could share it across
threads or tick entities in parallel, and it pays for that on every access even though nothing
uses it. This port gave that up; getting it back means `Sync` storages plus declared system
access, which is exactly what the missing typed-query layer would provide.

**Structural changes can't overlap iteration.** Spawn/despawn/insert take `&mut World`, so a
system collects entities, drops its guards, then applies (see `tick_items`). Consequences:
- Marking `Removed` and the final despawn are separate steps.
- steel-current's "spawned mid-tick first ticks next tick" snapshot rule does not carry over.
  Nothing can spawn mid-tick here (`spawn_*` needs `&mut GameWorld`). A system that wants to
  spawn (item drops, breeding) needs a deferred command queue; this crate has none.

**Invariants moved from types to `expect`.** `Mob: LivingEntity: Entity` guaranteed a mob has
position/velocity/tick count. Now a mob "should" have them; `tick_goal_selectors` panics with
`mob without Position` if not. Components must also be registered up front, else runtime panic.

**Order is manual and visible in the log.** Per-entity step order is now the order of
`add_system` calls. Cross-entity order changed: a system runs one step over *all* entities, and
dense storage order is not spawn order after any removal. Inline printing would regroup lines by
step. To keep stdout byte-identical, systems push to a `Logs` resource and the last system
stable-sorts by (entity id, then block entity id) before printing. That buffer exists purely to
reproduce the old observable behavior.

**Goals do not become data.** `Goal` trait objects keep private state (`remaining`, `direction`)
inside the boxed goal, outside the storages. A goal that needs a new component means extending
`MobCtx` and `tick_goal_selectors`. `Goal` is `&MobCtx`, not `&dyn Mob`: the old `as_animal()`
became `Option<&mut Hunger>`.

**Two ids per entity.** `Entity` is a recycled `(index, generation)` handle; the Steel-style id
in log lines must be monotonic and never reused, so it is its own `EntityId` component. Held
handles go stale on despawn, so every accessor is `Option` (`world.health(cow)` instead of
`cow.health()`); `Arc<T>` handles with typed methods are gone.

**Memory/shape details.** Each `SparseSet` keeps a sparse array sized to the highest entity
index ever stored for that component (20 storages here). Removal is `swap_remove`, so iteration
order is unstable. Despawn is O(number of component types).

**New entity kind = more touch points.** Components + `register_components` + system +
`build_schedule` entry + (if tests want it) a `GameWorld` accessor. Cow vs chicken is now just
`AnimalSpec` data, which is the part that got *cheaper*.

## Measured (release, 3 runs each, median, stdout to /dev/null)

Scratch benchmark outside the repo (not committed): N mixed entities (cow/chicken/item/falling
block) + N/5 block entities, 200 ticks, one thread everywhere. Both emit the same number of log
lines (178 500 at N = 10 000). `current` = a copy of `steel-current`, only `main` replaced.
`current, no locks` = that copy with `SyncMutex` swapped for an unsynchronized cell. It is an
ablation to attribute the gap, unsound under threads, never meant to ship.

| N entities | current | current, no locks | steel-ecs-ai |
|-----------:|--------:|------------------:|-------------:|
|     10 000 |  290 ms |            111 ms |        89 ms |
|    100 000 |  3.19 s |            1.40 s |       1.00 s |

So the ECS port is faster even though both tick on one thread. Where it comes from:
- **Mutex: ~85% of the gap.** Taking the lock out of steel-current alone goes 290 -> 111 ms
  (3.19 -> 1.40 s). It pays an uncontended `std::sync::Mutex` lock/unlock on every field access
  for thread-safety nothing uses today. Real Steel's `SyncMutex` may be cheaper than std's
  (not measured here), which would shrink this part.
- **Remainder: 22 ms at 10k, 0.4 s at 100k (current-no-locks is still 25-40% slower than the
  ECS).** Not separately measured. Candidates:
  `Arc<dyn Entity>` pointer chasing and dyn dispatch, the per-tick snapshot `clone()` of the
  entity list (an atomic refcount inc/dec per entity), versus packed arrays. `perf` is blocked
  on this machine, so this part is attribution by elimination, not profiling.

Caveats: synthetic workload, uncontended locks, one machine. Speed is not what the port costs;
the costs that matter are the ones above: ergonomics, runtime-checked borrows, lost
multi-thread headroom, deferred structural changes.

Code size, comments/blank/test lines excluded: steel-current ~880 lines; this port ~1030 lines
of game code (rustfmt expanded some lines, so slightly inflated) plus ~300 lines of ECS.
