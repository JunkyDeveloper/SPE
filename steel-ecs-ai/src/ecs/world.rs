use std::any::{Any, TypeId, type_name};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

use super::entity::{Entities, Entity};
use super::storage::{AnyStorage, SparseSet};

/// Entities + one type-erased storage per component type + singleton resources.
///
/// Every storage and resource sits in its own `RefCell`, so systems can hold several guards
/// at once through a shared `&World` (`read::<A>()` next to `write::<B>()`). That is runtime
/// borrow checking: asking for the same component type twice with a `write` panics instead of
/// failing to compile, and there is no static access analysis a scheduler could use to run
/// systems in parallel. `RefCell` also makes `World` `!Sync`.
///
/// Structural changes (spawn, despawn, insert, remove) take `&mut self`, so the compiler
/// guarantees no storage guard is alive while they run. Systems therefore collect what they
/// want to change, drop their guards, then apply it.
#[derive(Default)]
pub struct World {
    entities: Entities,
    storages: HashMap<TypeId, RefCell<Box<dyn AnyStorage>>>,
    resources: HashMap<TypeId, RefCell<Box<dyn Any>>>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    // ---- entities ----

    pub fn spawn_empty(&mut self) -> Entity {
        self.entities.alloc()
    }

    /// Strips the entity from *every* registered storage (O(number of component types)).
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.entities.free(entity) {
            return false;
        }
        for storage in self.storages.values_mut() {
            storage.get_mut().remove_entity(entity);
        }
        true
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.is_alive(entity)
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    // ---- components ----

    /// Components must be registered up front: `read`/`write` take `&self` and cannot create
    /// a storage lazily.
    pub fn register<T: 'static>(&mut self) -> &mut Self {
        self.storages
            .entry(TypeId::of::<T>())
            .or_insert_with(|| RefCell::new(Box::new(SparseSet::<T>::new())));
        self
    }

    fn cell<T: 'static>(&self) -> &RefCell<Box<dyn AnyStorage>> {
        self.storages
            .get(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("component `{}` is not registered", type_name::<T>()))
    }

    pub fn insert<T: 'static>(&mut self, entity: Entity, component: T) -> Option<T> {
        assert!(
            self.entities.is_alive(entity),
            "insert `{}` on a dead entity",
            type_name::<T>()
        );
        self.storages
            .get_mut(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("component `{}` is not registered", type_name::<T>()))
            .get_mut()
            .as_any_mut()
            .downcast_mut::<SparseSet<T>>()
            .expect("storage type mismatch")
            .insert(entity, component)
    }

    pub fn remove<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        self.storages
            .get_mut(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("component `{}` is not registered", type_name::<T>()))
            .get_mut()
            .as_any_mut()
            .downcast_mut::<SparseSet<T>>()
            .expect("storage type mismatch")
            .remove(entity)
    }

    /// Shared guard over the whole storage of `T`.
    pub fn read<T: 'static>(&self) -> Ref<'_, SparseSet<T>> {
        let guard = self.cell::<T>().try_borrow().unwrap_or_else(|_| {
            panic!(
                "component `{}` is already mutably borrowed",
                type_name::<T>()
            )
        });
        Ref::map(guard, |s| {
            s.as_any()
                .downcast_ref::<SparseSet<T>>()
                .expect("storage type mismatch")
        })
    }

    /// Exclusive guard over the whole storage of `T`.
    pub fn write<T: 'static>(&self) -> RefMut<'_, SparseSet<T>> {
        let guard = self
            .cell::<T>()
            .try_borrow_mut()
            .unwrap_or_else(|_| panic!("component `{}` is already borrowed", type_name::<T>()));
        RefMut::map(guard, |s| {
            s.as_any_mut()
                .downcast_mut::<SparseSet<T>>()
                .expect("storage type mismatch")
        })
    }

    /// Single-entity lookup. Holds a shared guard on the whole storage while the `Ref` lives.
    pub fn get<T: 'static>(&self, entity: Entity) -> Option<Ref<'_, T>> {
        Ref::filter_map(self.read::<T>(), |s| s.get(entity)).ok()
    }

    // ---- resources ----

    pub fn insert_resource<R: 'static>(&mut self, resource: R) {
        self.resources
            .insert(TypeId::of::<R>(), RefCell::new(Box::new(resource)));
    }

    fn resource_cell<R: 'static>(&self) -> &RefCell<Box<dyn Any>> {
        self.resources
            .get(&TypeId::of::<R>())
            .unwrap_or_else(|| panic!("resource `{}` does not exist", type_name::<R>()))
    }

    pub fn res<R: 'static>(&self) -> Ref<'_, R> {
        let guard = self.resource_cell::<R>().try_borrow().unwrap_or_else(|_| {
            panic!(
                "resource `{}` is already mutably borrowed",
                type_name::<R>()
            )
        });
        Ref::map(guard, |b| {
            (**b).downcast_ref::<R>().expect("resource type mismatch")
        })
    }

    pub fn res_mut<R: 'static>(&self) -> RefMut<'_, R> {
        let guard = self
            .resource_cell::<R>()
            .try_borrow_mut()
            .unwrap_or_else(|_| panic!("resource `{}` is already borrowed", type_name::<R>()));
        RefMut::map(guard, |b| {
            (**b).downcast_mut::<R>().expect("resource type mismatch")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Hp(i32);
    struct Tag;

    fn world() -> World {
        let mut w = World::new();
        w.register::<Hp>().register::<Tag>();
        w
    }

    #[test]
    fn despawn_strips_every_storage_and_invalidates_handle() {
        let mut w = world();
        let e = w.spawn_empty();
        w.insert(e, Hp(3));
        w.insert(e, Tag);
        assert!(w.despawn(e));
        assert!(!w.is_alive(e));
        assert!(w.read::<Hp>().is_empty());
        assert!(w.read::<Tag>().is_empty());
        assert!(w.get::<Hp>(e).is_none());
        assert!(!w.despawn(e));
    }

    #[test]
    fn different_types_can_be_borrowed_together() {
        let mut w = world();
        let e = w.spawn_empty();
        w.insert(e, Hp(3));
        w.insert(e, Tag);
        let tags = w.read::<Tag>();
        let mut hp = w.write::<Hp>();
        for (entity, h) in hp.iter_mut() {
            if tags.contains(entity) {
                h.0 += 1;
            }
        }
        drop((tags, hp));
        assert_eq!(w.get::<Hp>(e).unwrap().0, 4);
    }

    #[test]
    #[should_panic(expected = "already borrowed")]
    fn double_write_is_a_runtime_panic_not_a_compile_error() {
        let w = world();
        let _a = w.write::<Hp>();
        let _b = w.write::<Hp>();
    }

    #[test]
    #[should_panic(expected = "is not registered")]
    fn unregistered_component_panics() {
        struct Unregistered;
        let w = world();
        let _ = w.read::<Unregistered>();
    }

    #[test]
    fn resources_round_trip() {
        let mut w = world();
        w.insert_resource(41u64);
        *w.res_mut::<u64>() += 1;
        assert_eq!(*w.res::<u64>(), 42);
    }
}
