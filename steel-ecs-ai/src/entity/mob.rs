//! Steel: `mob/mod.rs`. Part: move speed, head yaw, goal selector. Stage: run the goals.

use std::mem;

use super::{Cx, Entity, GoalSelector};

pub struct Mob {
    /// Blocks per tick along x while walking.
    pub move_speed: f64,
    pub yaw: f32,
    pub goals: GoalSelector,
}

impl Mob {
    pub fn new(move_speed: f64) -> Self {
        Self {
            move_speed,
            yaw: 0.0,
            goals: GoalSelector::default(),
        }
    }

    /// Returns the new yaw.
    pub fn add_yaw(&mut self, delta: f32) -> f32 {
        self.yaw = (self.yaw + delta) % 360.0;
        self.yaw
    }
}

/// Steel: `Mob.serverAiStep` -> `tick_goal_selectors`.
///
/// The selector is moved out of the entity while it runs so that goals can take the whole
/// `&mut Entity` (position, velocity, other parts) without aliasing it. `entity.mob().goals` is
/// empty during that time; nothing else can observe it.
pub fn run_goals(e: &mut Entity, cx: &mut Cx) {
    let mut goals = mem::take(&mut e.mob_mut().goals);
    goals.tick(e, cx);
    e.mob_mut().goals = goals;
}
