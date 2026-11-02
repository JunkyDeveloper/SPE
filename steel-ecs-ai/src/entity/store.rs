//! Steel: `entity/manager` (the entity list) + the tick context entities use to reach the world.
//!
//! steel-current kept `Arc<dyn Entity>` in a `Mutex<Vec<_>>` and cloned the whole list every tick
//! to iterate it (the snapshot rule: things spawned mid-tick first tick next tick). Here the
//! world *owns* its entities (`Box<Entity>` in generational slots), and ticking takes `&mut`:
//!
//! - no `Arc`, no per-field mutex, no per-tick snapshot clone; the snapshot rule is "iterate
//!   `order[..len at tick start]`", spawned entities are appended behind that bound.
//! - while an entity ticks it is *moved out of its slot* ([`Store::take`]). That is what lets it
//!   hold `&mut Entity` and, at the same time, `&mut Cx` to reach every *other* entity: the two
//!   never alias. Consequences: an entity never sees itself through `Cx` (`get` on its own handle
//!   is `None`, `for_each_other` skips it), which is also what vanilla's pushing code wants.
//! - what it gives up: nothing can touch an entity from another thread while the world ticks.
//!   Steel's `Arc<dyn Entity>` + mutexes allow that (packet workers, chunk sender); here that
//!   would be a coarse lock around the whole `GameWorld` or a command queue. See the README.

use std::mem;

use super::{Entity, EntityId};

/// Reference to an entity in a [`Store`]. Recycled slots bump the generation, so a handle to a
/// despawned entity is `None` everywhere instead of pointing at a stranger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Handle {
    index: u32,
    generation: u32,
}

enum State {
    Free,
    /// Moved out for the duration of its own tick.
    Ticking,
    Live(Box<Entity>),
}

struct Slot {
    generation: u32,
    state: State,
}

#[derive(Default)]
pub struct Store {
    slots: Vec<Slot>,
    free: Vec<u32>,
    /// Spawn order = tick order (steel-current: `tick_list`).
    order: Vec<Handle>,
}

impl Store {
    /// All live handles in spawn order, including removed ones not yet dropped.
    pub fn handles(&self) -> &[Handle] {
        &self.order
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn contains(&self, handle: Handle) -> bool {
        self.slot(handle)
            .is_some_and(|slot| !matches!(slot.state, State::Free))
    }

    pub fn get(&self, handle: Handle) -> Option<&Entity> {
        match &self.slot(handle)?.state {
            State::Live(entity) => Some(entity),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, handle: Handle) -> Option<&mut Entity> {
        match &mut self.slot_mut(handle)?.state {
            State::Live(entity) => Some(entity),
            _ => None,
        }
    }

    fn slot(&self, handle: Handle) -> Option<&Slot> {
        self.slots
            .get(handle.index as usize)
            .filter(|slot| slot.generation == handle.generation)
    }

    fn slot_mut(&mut self, handle: Handle) -> Option<&mut Slot> {
        self.slots
            .get_mut(handle.index as usize)
            .filter(|slot| slot.generation == handle.generation)
    }

    pub(crate) fn insert(&mut self, entity: Entity) -> Handle {
        let entity = Box::new(entity);
        let handle = if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.state = State::Live(entity);
            Handle {
                index,
                generation: slot.generation,
            }
        } else {
            let index = u32::try_from(self.slots.len()).expect("more than u32::MAX entities");
            self.slots.push(Slot {
                generation: 0,
                state: State::Live(entity),
            });
            Handle {
                index,
                generation: 0,
            }
        };
        self.order.push(handle);
        handle
    }

    /// Moves an entity out for its own tick. `None` if it is gone or already out.
    pub(crate) fn take(&mut self, handle: Handle) -> Option<Box<Entity>> {
        let slot = self.slot_mut(handle)?;
        match mem::replace(&mut slot.state, State::Ticking) {
            State::Live(entity) => Some(entity),
            other => {
                slot.state = other;
                None
            }
        }
    }

    pub(crate) fn put(&mut self, handle: Handle, entity: Box<Entity>) {
        let slot = self
            .slot_mut(handle)
            .expect("entity slot vanished mid-tick");
        debug_assert!(matches!(slot.state, State::Ticking));
        slot.state = State::Live(entity);
    }

    /// Steel: `tick_list.retain(|e| !e.is_removed())`. Frees the slots, invalidating handles.
    pub(crate) fn drop_removed(&mut self) {
        let Self { slots, free, order } = self;
        order.retain(|handle| {
            let slot = &mut slots[handle.index as usize];
            let removed = matches!(&slot.state, State::Live(e) if e.base.removed);
            if removed {
                slot.state = State::Free;
                slot.generation += 1;
                free.push(handle.index);
            }
            !removed
        });
    }
}

/// What a running stage (or goal) can do besides touching its own entity.
/// Steel: `entity.level()` / the `World` APIs entity code calls.
pub struct Cx<'w> {
    pub(crate) time: u64,
    pub(crate) store: &'w mut Store,
    pub(crate) next_id: &'w mut u32,
}

impl Cx<'_> {
    /// Steel: `World::game_time`.
    pub fn time(&self) -> u64 {
        self.time
    }

    /// Adds an entity and gives it the next network id. It is appended behind the current tick's
    /// bound, so it first ticks next tick (steel-current: "spawned mid-tick first ticks next
    /// tick"), but other entities can see it right away.
    pub fn spawn(&mut self, mut entity: Entity) -> Handle {
        entity.base.id = EntityId(*self.next_id);
        *self.next_id += 1;
        self.store.insert(entity)
    }

    /// Another entity, if it is alive and not the one currently ticking.
    pub fn get(&self, handle: Handle) -> Option<&Entity> {
        self.store.get(handle)
    }

    pub fn get_mut(&mut self, handle: Handle) -> Option<&mut Entity> {
        self.store.get_mut(handle)
    }

    /// Visits every other entity in spawn order, changes visible immediately. A linear scan:
    /// the spot where Steel's spatial index (`get_entities_in_aabb`) would plug in.
    pub fn for_each_other(&mut self, mut f: impl FnMut(&mut Entity)) {
        let mut index = 0;
        while let Some(&handle) = self.store.order.get(index) {
            if let Some(entity) = self.store.get_mut(handle) {
                f(entity);
            }
            index += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Kind;
    use crate::util::Vec3;

    static KIND: Kind = Kind {
        name: "test",
        pipeline: &[],
    };

    fn entity() -> Entity {
        Entity::new(&KIND, Vec3::ZERO)
    }

    fn removed_entity() -> Entity {
        let mut e = entity();
        e.base.removed = true;
        e
    }

    #[test]
    fn stale_handle_is_none_after_slot_is_recycled() {
        let mut store = Store::default();
        let first = store.insert(removed_entity());
        store.drop_removed();
        assert!(!store.contains(first));

        let second = store.insert(entity());
        assert_ne!(first, second, "same slot, new generation");
        assert!(store.get(first).is_none());
        assert!(store.get(second).is_some());
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn taken_entity_is_invisible_but_alive() {
        let mut store = Store::default();
        let a = store.insert(entity());
        let b = store.insert(entity());

        let out = store.take(a).unwrap();
        assert!(store.get(a).is_none(), "not reachable while it ticks");
        assert!(store.contains(a), "but still alive");
        assert!(store.take(a).is_none(), "can't be taken twice");
        assert!(store.get(b).is_some());

        store.put(a, out);
        assert!(store.get(a).is_some());
    }

    #[test]
    fn drop_removed_keeps_spawn_order() {
        let mut store = Store::default();
        let a = store.insert(entity());
        let _b = store.insert(removed_entity());
        let c = store.insert(entity());
        store.drop_removed();
        assert_eq!(store.handles(), [a, c]);
    }
}
