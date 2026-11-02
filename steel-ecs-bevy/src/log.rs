//! Deferred, deterministically ordered log output.
//!
//! steel-current ticks one entity at a time and prints inline, so all lines of entity A come
//! before any line of entity B. Here a system runs one tick *step* over every entity, and
//! bevy's query iteration order is archetype/table order, not spawn order (and changes when
//! components are added or removed), so inline printing would regroup the output by step and by
//! table layout. Systems push into the [`Logs`] resource instead; the last system of the tick
//! stable-sorts by source (entities by id, then block entities by id) and prints. Lines of one
//! source keep their emission order, which is the step order, so stdout is byte-identical to
//! steel-current. Bevy does not remove the need for this buffer.

use bevy_ecs::prelude::Resource;

use crate::block_entity::BlockEntityId;
use crate::entity::EntityId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogSource {
    Entity(EntityId),
    BlockEntity(BlockEntityId),
}

#[derive(Resource, Default)]
pub struct Logs {
    pending: Vec<(LogSource, String)>,
}

impl Logs {
    pub fn push(&mut self, source: LogSource, line: String) {
        self.pending.push((source, line));
    }

    /// `[ tick] name#id: msg`
    pub fn entity(&mut self, id: EntityId, name: &str, tick_count: u32, msg: &str) {
        self.push(
            LogSource::Entity(id),
            format!("[{tick_count:>4}] {name}#{}: {msg}", id.0),
        );
    }

    /// `[ tick] name@x,y,z: msg`
    pub fn block_entity(
        &mut self,
        id: BlockEntityId,
        name: &str,
        pos: (i32, i32, i32),
        tick_count: u32,
        msg: &str,
    ) {
        let (x, y, z) = pos;
        self.push(
            LogSource::BlockEntity(id),
            format!("[{tick_count:>4}] {name}@{x},{y},{z}: {msg}"),
        );
    }

    /// Takes all pending lines, grouped by source, emission order kept within a source.
    pub fn drain_sorted(&mut self) -> Vec<String> {
        self.pending.sort_by_key(|(source, _)| *source); // stable
        self.pending.drain(..).map(|(_, line)| line).collect()
    }

    pub fn flush(&mut self) {
        for line in self.drain_sorted() {
            println!("{line}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_by_source_and_keeps_emission_order_within_one() {
        let mut logs = Logs::default();
        logs.block_entity(BlockEntityId(0), "campfire", (1, 2, 3), 7, "late");
        logs.entity(EntityId(1), "b", 7, "step 1");
        logs.entity(EntityId(0), "a", 7, "step 1");
        logs.entity(EntityId(1), "b", 7, "step 2");
        logs.entity(EntityId(0), "a", 7, "step 2");
        assert_eq!(
            logs.drain_sorted(),
            [
                "[   7] a#0: step 1",
                "[   7] a#0: step 2",
                "[   7] b#1: step 1",
                "[   7] b#1: step 2",
                "[   7] campfire@1,2,3: late",
            ]
        );
        assert!(logs.drain_sorted().is_empty());
    }
}
