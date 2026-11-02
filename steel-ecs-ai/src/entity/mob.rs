//! Steel: `mob/mod.rs`. Data: goal selector + head yaw (+ walking speed). Tick step: run the goals.

use crate::ecs::World;
use crate::entity::{EntityHeader, EntityId, GoalSelector, Hunger, Position, Velocity};
use crate::log::Logs;
use crate::util::Vec3;

/// Blocks per tick along x while walking. (steel-current: `Mob::move_speed()` trait method.)
pub struct MoveSpeed(pub f64);

pub struct Yaw(pub f32);

/// What a goal sees of its mob. Replaces `&dyn Mob`.
///
/// In steel-current a goal got `&dyn Mob` and went through `&self` accessors backed by
/// mutexes. Here there are no locks: the selector system borrows the needed storages once
/// per tick and hands each goal plain `&mut` references to *its* entity's data. The set of
/// fields is the set of things goals may touch; a new goal that needs another component means
/// extending this struct and the system below.
pub struct MobCtx<'a> {
    pub id: EntityId,
    pub name: &'a str,
    pub tick_count: u32,
    pub move_speed: f64,
    pub position: &'a mut Vec3,
    pub velocity: &'a mut Vec3,
    pub yaw: &'a mut f32,
    /// Steel: `mob.as_animal()`.
    pub hunger: Option<&'a mut Hunger>,
    pub logs: &'a mut Logs,
}

impl MobCtx<'_> {
    pub fn log(&mut self, msg: &str) {
        self.logs.entity(self.id, self.name, self.tick_count, msg);
    }

    /// Returns the new yaw.
    pub fn add_yaw(&mut self, delta: f32) -> f32 {
        *self.yaw = (*self.yaw + delta) % 360.0;
        *self.yaw
    }
}

/// Steel: `Mob.serverAiStep` -> `tick_goal_selectors`.
pub fn tick_goal_selectors(world: &mut World) {
    let header = EntityHeader::new(world);
    let speeds = world.read::<MoveSpeed>();
    let mut selectors = world.write::<GoalSelector>();
    let mut positions = world.write::<Position>();
    let mut velocities = world.write::<Velocity>();
    let mut yaws = world.write::<Yaw>();
    let mut hungers = world.write::<Hunger>();
    let mut logs = world.res_mut::<Logs>();

    for (entity, selector) in selectors.iter_mut() {
        if !header.is_active(entity) {
            continue;
        }
        let mut mob = MobCtx {
            id: header.id(entity),
            name: header.name(entity),
            tick_count: header.tick_count(entity),
            move_speed: speeds.get(entity).expect("mob without MoveSpeed").0,
            position: &mut positions.get_mut(entity).expect("mob without Position").0,
            velocity: &mut velocities.get_mut(entity).expect("mob without Velocity").0,
            yaw: &mut yaws.get_mut(entity).expect("mob without Yaw").0,
            hunger: hungers.get_mut(entity),
            logs: &mut logs,
        };
        selector.tick(&mut mob);
    }
}
