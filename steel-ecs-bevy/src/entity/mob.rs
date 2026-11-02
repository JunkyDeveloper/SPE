//! Steel: `mob/mod.rs`. Data: goal selector + head yaw (+ walking speed). Tick step: run the goals.

use bevy_ecs::prelude::{Component, Query, ResMut, Without};

use crate::entity::{EntityId, GoalSelector, Hunger, Name, Position, Removed, TickCount, Velocity};
use crate::log::Logs;
use crate::util::Vec3;

/// Blocks per tick along x while walking. (steel-current: `Mob::move_speed()` trait method.)
#[derive(Component)]
pub struct MoveSpeed(pub f64);

#[derive(Component)]
pub struct Yaw(pub f32);

/// What a goal sees of its mob. Replaces `&dyn Mob`.
///
/// Same struct as in steel-ecs-ai: built per entity from the query item, handing each goal
/// plain `&mut` references to its entity's data. Bevy's `Mut<T>` wrappers are dereferenced
/// here, so writing through `position`/`velocity`/`yaw` flags those components as changed
/// (bevy's change detection; unused by this port).
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
///
/// One query replaces steel-ecs-ai's nine hand-taken guards plus `EntityHeader`. `Option<&mut
/// Hunger>` is the query-level spelling of steel-current's `mob.as_animal()`.
#[allow(clippy::type_complexity)]
pub fn tick_goal_selectors(
    mut logs: ResMut<Logs>,
    mut mobs: Query<
        (
            &EntityId,
            &Name,
            &TickCount,
            &MoveSpeed,
            &mut GoalSelector,
            &mut Position,
            &mut Velocity,
            &mut Yaw,
            Option<&mut Hunger>,
        ),
        Without<Removed>,
    >,
) {
    for (id, name, tick, speed, mut selector, mut position, mut velocity, mut yaw, mut hunger) in
        &mut mobs
    {
        let mut mob = MobCtx {
            id: *id,
            name: name.0,
            tick_count: tick.0,
            move_speed: speed.0,
            position: &mut position.0,
            velocity: &mut velocity.0,
            yaw: &mut yaw.0,
            hunger: hunger.as_deref_mut(),
            logs: &mut logs,
        };
        selector.tick(&mut mob);
    }
}
