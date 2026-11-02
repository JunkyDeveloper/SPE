//! Sparse-set component storage: one `SparseSet<T>` per component type.
//!
//! ```text
//! sparse: entity index -> dense index          (grows to the highest entity index ever stored)
//! dense:  dense index  -> Entity               (packed)
//! values: dense index  -> T                    (packed, same order as `dense`)
//! ```
//!
//! Iteration is a linear walk over the packed arrays; lookup/insert/remove are O(1); removal is
//! `swap_remove`, so **iteration order is not insertion order** once anything has been removed.

use std::any::Any;

use super::entity::Entity;

const NONE: u32 = u32::MAX;

pub struct SparseSet<T> {
    sparse: Vec<u32>,
    dense: Vec<Entity>,
    values: Vec<T>,
}

impl<T> Default for SparseSet<T> {
    fn default() -> Self {
        Self {
            sparse: Vec::new(),
            dense: Vec::new(),
            values: Vec::new(),
        }
    }
}

impl<T> SparseSet<T> {
    pub fn new() -> Self {
        Self::default()
    }

    fn dense_index(&self, entity: Entity) -> Option<usize> {
        let dense = *self.sparse.get(entity.index())?;
        // Compare the full handle: a stale generation must not match.
        (dense != NONE && self.dense[dense as usize] == entity).then_some(dense as usize)
    }

    /// Returns the previous value if the entity already had this component.
    pub fn insert(&mut self, entity: Entity, value: T) -> Option<T> {
        if let Some(dense) = self.dense_index(entity) {
            return Some(std::mem::replace(&mut self.values[dense], value));
        }
        if self.sparse.len() <= entity.index() {
            self.sparse.resize(entity.index() + 1, NONE);
        }
        debug_assert_eq!(
            self.sparse[entity.index()],
            NONE,
            "stale entry left behind by a despawn"
        );
        self.sparse[entity.index()] = u32::try_from(self.dense.len()).expect("storage overflow");
        self.dense.push(entity);
        self.values.push(value);
        None
    }

    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let dense = self.dense_index(entity)?;
        self.sparse[entity.index()] = NONE;
        self.dense.swap_remove(dense);
        let value = self.values.swap_remove(dense);
        if let Some(&moved) = self.dense.get(dense) {
            self.sparse[moved.index()] = dense as u32;
        }
        Some(value)
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.dense_index(entity).is_some()
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.dense_index(entity).map(|i| &self.values[i])
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.dense_index(entity).map(|i| &mut self.values[i])
    }

    pub fn len(&self) -> usize {
        self.dense.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    pub fn entities(&self) -> &[Entity] {
        &self.dense
    }

    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.dense.iter().copied().zip(self.values.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.dense.iter().copied().zip(self.values.iter_mut())
    }
}

/// Type-erased view of a `SparseSet<T>`: what the world can do without knowing `T`.
/// Despawning has to go through this to strip an entity from every storage.
pub trait AnyStorage: Any {
    fn remove_entity(&mut self, entity: Entity);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static> AnyStorage for SparseSet<T> {
    fn remove_entity(&mut self, entity: Entity) {
        self.remove(entity);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::entity::Entities;

    #[test]
    fn swap_remove_keeps_moved_entity_reachable() {
        let mut entities = Entities::default();
        let (a, b, c) = (entities.alloc(), entities.alloc(), entities.alloc());
        let mut set = SparseSet::new();
        set.insert(a, "a");
        set.insert(b, "b");
        set.insert(c, "c");

        assert_eq!(set.remove(a), Some("a"));
        assert_eq!(set.get(c), Some(&"c"), "c was swapped into a's dense slot");
        assert_eq!(set.get(b), Some(&"b"));
        assert!(!set.contains(a));
        assert_eq!(set.len(), 2);
        // Order changed: that is the documented cost of swap_remove.
        assert_eq!(set.entities(), &[c, b]);
    }

    #[test]
    fn insert_replaces_and_stale_handles_miss() {
        let mut entities = Entities::default();
        let a = entities.alloc();
        let mut set = SparseSet::new();
        assert_eq!(set.insert(a, 1), None);
        assert_eq!(set.insert(a, 2), Some(1));
        set.remove(a);
        entities.free(a);
        let reused = entities.alloc();
        set.insert(reused, 3);
        assert_eq!(
            set.get(a),
            None,
            "old generation must not see the new value"
        );
        assert_eq!(set.get(reused), Some(&3));
    }
}
