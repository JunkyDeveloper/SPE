//! Steel: `mob/mod.rs`. Data: goal selector + head yaw. Tick step: run the goals.

use crate::entity::{Goal, GoalSelector, LivingEntity};
use crate::util::SyncMutex;

pub struct MobBase {
    goal_selector: SyncMutex<GoalSelector>,
    yaw: SyncMutex<f32>,
}

impl MobBase {
    pub fn new() -> Self {
        Self { goal_selector: SyncMutex::new(GoalSelector::default()), yaw: SyncMutex::new(0.0) }
    }
    pub fn goal_selector(&self) -> &SyncMutex<GoalSelector> {
        &self.goal_selector
    }
    pub fn add_goal(&self, priority: i32, goal: Box<dyn Goal>) {
        self.goal_selector.lock().add_goal(priority, goal);
    }
    pub fn yaw(&self) -> f32 {
        *self.yaw.lock()
    }
    /// Returns the new yaw.
    pub fn add_yaw(&self, delta: f32) -> f32 {
        let mut y = self.yaw.lock();
        *y = (*y + delta) % 360.0;
        *y
    }
}

pub trait Mob: LivingEntity {
    fn mob_base(&self) -> &MobBase;
    /// Blocks per tick along x while walking.
    fn move_speed(&self) -> f64;

    fn yaw(&self) -> f32 {
        self.mob_base().yaw()
    }

    /// Steel: `Mob.serverAiStep` -> `tick_goal_selectors`.
    fn mob_ai_step(&self) {
        if let Some(mob) = self.as_mob() {
            self.mob_base().goal_selector().lock().tick(mob);
        }
    }
}
