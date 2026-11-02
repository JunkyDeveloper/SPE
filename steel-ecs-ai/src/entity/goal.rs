//! Steel: `entity/ai/goal/selector.rs` + individual goals, reduced to four deterministic goals:
//! shared `WanderGoal`, `LookAroundGoal`; cow-only `EatGrassGoal`; chicken-only `HopGoal`.
//!
//! Lower priority number = more important. Goals whose flags overlap cannot run together:
//! a more important goal preempts (stops) a running conflicting one.
//!
//! Goals stay trait objects, owned by a `GoalSelector` *component*. Their per-instance state
//! (`remaining`, `direction`) lives inside the boxed goal, outside the ECS storages: goals
//! are the one part of the design that does not become plain data.

use crate::entity::MobCtx;
use crate::util::Vec3;

pub const FLAG_MOVE: u8 = 1;
pub const FLAG_LOOK: u8 = 2;
pub const FLAG_JUMP: u8 = 4;

pub trait Goal {
    fn flags(&self) -> u8;
    fn can_use(&mut self, mob: &MobCtx) -> bool;
    fn can_continue_to_use(&mut self, mob: &MobCtx) -> bool {
        self.can_use(mob)
    }
    fn start(&mut self, _mob: &mut MobCtx) {}
    fn stop(&mut self, _mob: &mut MobCtx) {}
    fn tick(&mut self, _mob: &mut MobCtx) {}
}

struct WrappedGoal {
    priority: i32,
    goal: Box<dyn Goal>,
    running: bool,
}

#[derive(Default)]
pub struct GoalSelector {
    goals: Vec<WrappedGoal>,
}

impl GoalSelector {
    pub fn add_goal(&mut self, priority: i32, goal: Box<dyn Goal>) {
        self.goals.push(WrappedGoal {
            priority,
            goal,
            running: false,
        });
        self.goals.sort_by_key(|g| g.priority);
    }

    /// Steel: `GoalSelector.tick`.
    pub fn tick(&mut self, mob: &mut MobCtx) {
        // 1. stop goals that can't continue
        for g in &mut self.goals {
            if g.running && !g.goal.can_continue_to_use(mob) {
                g.goal.stop(mob);
                g.running = false;
            }
        }
        // 2. start goals by priority; preempt less important conflicting ones
        for i in 0..self.goals.len() {
            if self.goals[i].running {
                continue;
            }
            let flags = self.goals[i].goal.flags();
            let priority = self.goals[i].priority;
            let blocked = self
                .goals
                .iter()
                .any(|o| o.running && o.goal.flags() & flags != 0 && o.priority <= priority);
            if blocked || !self.goals[i].goal.can_use(mob) {
                continue;
            }
            for j in 0..self.goals.len() {
                if j != i && self.goals[j].running && self.goals[j].goal.flags() & flags != 0 {
                    self.goals[j].goal.stop(mob);
                    self.goals[j].running = false;
                }
            }
            self.goals[i].goal.start(mob);
            self.goals[i].running = true;
        }
        // 3. tick running goals
        for g in &mut self.goals {
            if g.running {
                g.goal.tick(mob);
            }
        }
    }
}

/// Shared. Every 30 ticks walk along x for 10 ticks, alternating direction.
pub struct WanderGoal {
    remaining: u32,
    direction: f64,
}

impl WanderGoal {
    pub fn new() -> Self {
        Self {
            remaining: 0,
            direction: -1.0,
        }
    }
}

impl Default for WanderGoal {
    fn default() -> Self {
        Self::new()
    }
}

impl Goal for WanderGoal {
    fn flags(&self) -> u8 {
        FLAG_MOVE
    }
    fn can_use(&mut self, mob: &MobCtx) -> bool {
        mob.tick_count.is_multiple_of(30)
    }
    fn can_continue_to_use(&mut self, _mob: &MobCtx) -> bool {
        self.remaining > 0
    }
    fn start(&mut self, mob: &mut MobCtx) {
        self.remaining = 10;
        self.direction = -self.direction;
        mob.log(if self.direction > 0.0 {
            "wanders east"
        } else {
            "wanders west"
        });
    }
    fn tick(&mut self, mob: &mut MobCtx) {
        self.remaining -= 1;
        mob.velocity.x = self.direction * mob.move_speed;
    }
    fn stop(&mut self, mob: &mut MobCtx) {
        mob.velocity.x = 0.0;
    }
}

/// Shared. Every 15 ticks turn the head by 45 degrees over 5 ticks.
pub struct LookAroundGoal {
    remaining: u32,
}

impl LookAroundGoal {
    pub fn new() -> Self {
        Self { remaining: 0 }
    }
}

impl Default for LookAroundGoal {
    fn default() -> Self {
        Self::new()
    }
}

impl Goal for LookAroundGoal {
    fn flags(&self) -> u8 {
        FLAG_LOOK
    }
    fn can_use(&mut self, mob: &MobCtx) -> bool {
        mob.tick_count.is_multiple_of(15)
    }
    fn can_continue_to_use(&mut self, _mob: &MobCtx) -> bool {
        self.remaining > 0
    }
    fn start(&mut self, _mob: &mut MobCtx) {
        self.remaining = 5;
    }
    fn tick(&mut self, mob: &mut MobCtx) {
        self.remaining -= 1;
        let yaw = mob.add_yaw(9.0);
        if self.remaining == 0 {
            mob.log(&format!("looked around, yaw {yaw}"));
        }
    }
}

/// Cow only. When hungry, stand still for 5 ticks, then hunger resets. Preempts wander/look.
pub struct EatGrassGoal {
    remaining: u32,
}

impl EatGrassGoal {
    pub fn new() -> Self {
        Self { remaining: 0 }
    }
}

impl Default for EatGrassGoal {
    fn default() -> Self {
        Self::new()
    }
}

impl Goal for EatGrassGoal {
    fn flags(&self) -> u8 {
        FLAG_MOVE | FLAG_LOOK
    }
    fn can_use(&mut self, mob: &MobCtx) -> bool {
        mob.hunger.as_deref().is_some_and(|h| h.is_hungry())
    }
    fn can_continue_to_use(&mut self, _mob: &MobCtx) -> bool {
        self.remaining > 0
    }
    fn start(&mut self, mob: &mut MobCtx) {
        self.remaining = 5;
        mob.log("starts eating grass");
    }
    fn tick(&mut self, mob: &mut MobCtx) {
        self.remaining -= 1;
        *mob.velocity = Vec3::ZERO;
        if self.remaining == 0 {
            if let Some(hunger) = mob.hunger.as_deref_mut() {
                hunger.reset();
            }
            mob.log("ate grass");
        }
    }
}

/// Chicken only. Every 40 ticks hop: +0.25 y for 2 ticks, then -0.25 y for 2 ticks.
pub struct HopGoal {
    remaining: u32,
}

impl HopGoal {
    pub fn new() -> Self {
        Self { remaining: 0 }
    }
}

impl Default for HopGoal {
    fn default() -> Self {
        Self::new()
    }
}

impl Goal for HopGoal {
    fn flags(&self) -> u8 {
        FLAG_JUMP
    }
    fn can_use(&mut self, mob: &MobCtx) -> bool {
        mob.tick_count.is_multiple_of(40)
    }
    fn can_continue_to_use(&mut self, _mob: &MobCtx) -> bool {
        self.remaining > 0
    }
    fn start(&mut self, _mob: &mut MobCtx) {
        self.remaining = 4;
    }
    fn tick(&mut self, mob: &mut MobCtx) {
        mob.position.y += if self.remaining > 2 { 0.25 } else { -0.25 };
        self.remaining -= 1;
        if self.remaining == 0 {
            mob.log("hopped");
        }
    }
}
