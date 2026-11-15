//! Steel: `entity/ai/goal/selector.rs`. Same algorithm:
//! 1. stop running goals that can't continue,
//! 2. start usable goals whose controls are free or held by a less important interruptable goal,
//! 3. tick running goals.
//!
//! Lower priority number = more important.

use std::ops::BitOr;

use crate::entity::{PathfinderMob, SharedEntity};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GoalControls(u8);

impl GoalControls {
    pub const EMPTY: Self = Self(0);
    pub const MOVE: Self = Self(1 << 0);
    pub const LOOK: Self = Self(1 << 1);
    const ALL: [Self; 2] = [Self::MOVE, Self::LOOK];

    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
    fn iter(self) -> impl Iterator<Item = Self> {
        Self::ALL.into_iter().filter(move |c| self.intersects(*c))
    }
}

impl BitOr for GoalControls {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Vanilla `Goal.reducedTickDelay`: goals not ticked every tick count in half-ticks.
pub const fn reduced_tick_delay(ticks: i32) -> i32 {
    (ticks + 1) / 2
}

pub trait Goal: Send {
    fn controls(&self) -> GoalControls;
    fn can_use(&mut self, mob: &dyn PathfinderMob) -> bool;
    fn can_continue_to_use(&mut self, mob: &dyn PathfinderMob) -> bool {
        self.can_use(mob)
    }
    fn is_interruptable(&self) -> bool {
        true
    }
    fn start(&mut self, _mob: &dyn PathfinderMob) {}
    fn stop(&mut self, _mob: &dyn PathfinderMob) {}
    fn requires_update_every_tick(&self) -> bool {
        false
    }
    fn tick(&mut self, _mob: &dyn PathfinderMob, _entity: &SharedEntity) {}
}

struct WrappedGoal {
    priority: i32,
    goal: Box<dyn Goal>,
    running: bool,
}

impl WrappedGoal {
    fn can_be_replaced_by(&self, priority: i32) -> bool {
        self.goal.is_interruptable() && priority < self.priority
    }
    fn start(&mut self, mob: &dyn PathfinderMob) {
        if !self.running {
            self.running = true;
            self.goal.start(mob);
        }
    }
    fn stop(&mut self, mob: &dyn PathfinderMob) {
        if self.running {
            self.running = false;
            self.goal.stop(mob);
        }
    }
}

#[derive(Default)]
pub struct GoalSelector {
    available_goals: Vec<WrappedGoal>,
}

impl GoalSelector {
    pub fn add_goal<G: Goal + 'static>(&mut self, priority: i32, goal: G) {
        self.available_goals.push(WrappedGoal { priority, goal: Box::new(goal), running: false });
    }

    pub fn tick(&mut self, mob: &dyn PathfinderMob, entity: &SharedEntity) {
        for g in &mut self.available_goals {
            if g.running && !g.goal.can_continue_to_use(mob) {
                g.stop(mob);
            }
        }

        for i in 0..self.available_goals.len() {
            if !self.can_start_goal(i) || !self.available_goals[i].goal.can_use(mob) {
                continue;
            }
            for control in self.available_goals[i].goal.controls().iter() {
                if let Some(current) = self.running_goal_index_for(control) {
                    self.available_goals[current].stop(mob);
                }
            }
            self.available_goals[i].start(mob);
        }

        self.tick_running_goals(mob, entity, true);
    }

    pub fn tick_running_goals(&mut self, mob: &dyn PathfinderMob, entity: &SharedEntity, force_all: bool) {
        for g in &mut self.available_goals {
            if g.running && (force_all || g.goal.requires_update_every_tick()) {
                g.goal.tick(mob, entity);
            }
        }
    }

    pub fn running_goal_count(&self) -> usize {
        self.available_goals.iter().filter(|g| g.running).count()
    }

    fn can_start_goal(&self, index: usize) -> bool {
        let candidate = &self.available_goals[index];
        !candidate.running
            && candidate.goal.controls().iter().all(|control| {
                self.running_goal_index_for(control)
                    .is_none_or(|current| self.available_goals[current].can_be_replaced_by(candidate.priority))
            })
    }

    fn running_goal_index_for(&self, control: GoalControls) -> Option<usize> {
        self.available_goals.iter().position(|g| g.running && g.goal.controls().intersects(control))
    }
}
