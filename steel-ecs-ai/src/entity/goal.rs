//! Steel: `entity/ai/goal/selector.rs` + individual goals, reduced to four deterministic goals:
//! shared `WanderGoal`, `LookAroundGoal`; cow-only `EatGrassGoal`; chicken-only `HopGoal`.
//!
//! Lower priority number = more important. Goals whose flags overlap cannot run together:
//! a more important goal preempts (stops) a running conflicting one.
//!
//! Goals stay trait objects: each keeps private per-instance state (`remaining`, `direction`),
//! which is not worth turning into data. They get the whole `&mut Entity` (instead of
//! steel-current's `&dyn Mob` accessor surface) and the tick context, so a goal can read or
//! change any part and, through `Cx`, other entities.

use super::{Animal, Cx, Entity};
use crate::util::Vec3;

pub const FLAG_MOVE: u8 = 1;
pub const FLAG_LOOK: u8 = 2;
pub const FLAG_JUMP: u8 = 4;

pub trait Goal: Send + Sync {
    fn flags(&self) -> u8;
    fn can_use(&mut self, e: &Entity, cx: &Cx) -> bool;
    fn can_continue_to_use(&mut self, e: &Entity, cx: &Cx) -> bool {
        self.can_use(e, cx)
    }
    fn start(&mut self, _e: &mut Entity, _cx: &mut Cx) {}
    fn stop(&mut self, _e: &mut Entity, _cx: &mut Cx) {}
    fn tick(&mut self, _e: &mut Entity, _cx: &mut Cx) {}
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
    pub fn add_goal(&mut self, priority: i32, goal: impl Goal + 'static) {
        self.goals.push(WrappedGoal {
            priority,
            goal: Box::new(goal),
            running: false,
        });
        self.goals.sort_by_key(|g| g.priority);
    }

    /// Steel: `GoalSelector.tick`.
    pub fn tick(&mut self, e: &mut Entity, cx: &mut Cx) {
        // 1. stop goals that can't continue
        for g in &mut self.goals {
            if g.running && !g.goal.can_continue_to_use(e, cx) {
                g.goal.stop(e, cx);
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
            if blocked || !self.goals[i].goal.can_use(e, cx) {
                continue;
            }
            for j in 0..self.goals.len() {
                if j != i && self.goals[j].running && self.goals[j].goal.flags() & flags != 0 {
                    self.goals[j].goal.stop(e, cx);
                    self.goals[j].running = false;
                }
            }
            self.goals[i].goal.start(e, cx);
            self.goals[i].running = true;
        }
        // 3. tick running goals
        for g in &mut self.goals {
            if g.running {
                g.goal.tick(e, cx);
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
    fn can_use(&mut self, e: &Entity, _cx: &Cx) -> bool {
        e.base.tick_count.is_multiple_of(30)
    }
    fn can_continue_to_use(&mut self, _e: &Entity, _cx: &Cx) -> bool {
        self.remaining > 0
    }
    fn start(&mut self, e: &mut Entity, _cx: &mut Cx) {
        self.remaining = 10;
        self.direction = -self.direction;
        e.log(if self.direction > 0.0 {
            "wanders east"
        } else {
            "wanders west"
        });
    }
    fn tick(&mut self, e: &mut Entity, _cx: &mut Cx) {
        self.remaining -= 1;
        e.base.velocity.x = self.direction * e.mob().move_speed;
    }
    fn stop(&mut self, e: &mut Entity, _cx: &mut Cx) {
        e.base.velocity.x = 0.0;
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
    fn can_use(&mut self, e: &Entity, _cx: &Cx) -> bool {
        e.base.tick_count.is_multiple_of(15)
    }
    fn can_continue_to_use(&mut self, _e: &Entity, _cx: &Cx) -> bool {
        self.remaining > 0
    }
    fn start(&mut self, _e: &mut Entity, _cx: &mut Cx) {
        self.remaining = 5;
    }
    fn tick(&mut self, e: &mut Entity, _cx: &mut Cx) {
        self.remaining -= 1;
        let yaw = e.mob_mut().add_yaw(9.0);
        if self.remaining == 0 {
            e.log(format_args!("looked around, yaw {yaw}"));
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
    fn can_use(&mut self, e: &Entity, _cx: &Cx) -> bool {
        e.animal.as_ref().is_some_and(Animal::is_hungry)
    }
    fn can_continue_to_use(&mut self, _e: &Entity, _cx: &Cx) -> bool {
        self.remaining > 0
    }
    fn start(&mut self, e: &mut Entity, _cx: &mut Cx) {
        self.remaining = 5;
        e.log("starts eating grass");
    }
    fn tick(&mut self, e: &mut Entity, _cx: &mut Cx) {
        self.remaining -= 1;
        e.base.velocity = Vec3::ZERO;
        if self.remaining == 0 {
            if let Some(animal) = e.animal.as_mut() {
                animal.reset_hunger();
            }
            e.log("ate grass");
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
    fn can_use(&mut self, e: &Entity, _cx: &Cx) -> bool {
        e.base.tick_count.is_multiple_of(40)
    }
    fn can_continue_to_use(&mut self, _e: &Entity, _cx: &Cx) -> bool {
        self.remaining > 0
    }
    fn start(&mut self, _e: &mut Entity, _cx: &mut Cx) {
        self.remaining = 4;
    }
    fn tick(&mut self, e: &mut Entity, _cx: &mut Cx) {
        e.base.position.y += if self.remaining > 2 { 0.25 } else { -0.25 };
        self.remaining -= 1;
        if self.remaining == 0 {
            e.log("hopped");
        }
    }
}
