//! Generational entity handles and their allocator.
//!
//! An `Entity` is just `(index, generation)`. The index addresses slots in the component
//! storages; the generation makes handles to despawned entities detectably stale instead of
//! silently aliasing whatever entity reuses the slot.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    pub fn index(self) -> usize {
        self.index as usize
    }

    pub fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Default)]
pub struct Entities {
    /// Current generation per slot. A handle is alive iff its generation matches.
    generations: Vec<u32>,
    free: Vec<u32>,
}

impl Entities {
    pub fn alloc(&mut self) -> Entity {
        if let Some(index) = self.free.pop() {
            Entity {
                index,
                generation: self.generations[index as usize],
            }
        } else {
            let index = u32::try_from(self.generations.len()).expect("entity index overflow");
            self.generations.push(0);
            Entity {
                index,
                generation: 0,
            }
        }
    }

    /// Returns `false` if the handle was already stale.
    pub fn free(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        let generation = &mut self.generations[entity.index()];
        *generation = generation.wrapping_add(1);
        self.free.push(entity.index);
        true
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.generations.get(entity.index()) == Some(&entity.generation)
    }

    pub fn len(&self) -> usize {
        self.generations.len() - self.free.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reused_slot_gets_new_generation_and_old_handle_is_stale() {
        let mut entities = Entities::default();
        let a = entities.alloc();
        assert!(entities.free(a));
        assert!(!entities.is_alive(a));
        assert!(!entities.free(a), "double free is rejected");

        let b = entities.alloc();
        assert_eq!(a.index(), b.index(), "slot reused");
        assert_ne!(a.generation(), b.generation());
        assert!(entities.is_alive(b));
        assert!(
            !entities.is_alive(a),
            "stale handle does not alias the new entity"
        );
        assert_eq!(entities.len(), 1);
    }
}
