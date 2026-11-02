use super::world::World;

/// A system is a plain function over the whole world.
pub type System = fn(&mut World);

/// Ordered list of systems, run one after another. There is no dependency graph, no
/// automatic ordering and no parallelism: the order of `add_system` calls *is* the semantics.
#[derive(Default)]
pub struct Schedule {
    systems: Vec<System>,
}

impl Schedule {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_system(&mut self, system: System) -> &mut Self {
        self.systems.push(system);
        self
    }

    pub fn run(&self, world: &mut World) {
        for system in &self.systems {
            system(world);
        }
    }

    pub fn len(&self) -> usize {
        self.systems.len()
    }

    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }
}
