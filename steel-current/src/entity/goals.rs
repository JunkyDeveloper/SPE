//! Steel: `entity/ai/goal/*`. The goals cows and chickens share in Steel, minus the ones that
//! need players or water (Float, Tempt, LookAtPlayer). Vanilla random rolls are replaced by
//! fixed, id-staggered intervals so runs are deterministic.

use crate::entity::*;
use crate::util::Vec3;

fn navigation_is_done(mob: &dyn PathfinderMob) -> bool {
    mob.mob_base().navigation().lock().is_done()
}

fn move_to(mob: &dyn PathfinderMob, target: Vec3, speed_modifier: f64) {
    mob.mob_base().navigation().lock().move_to(target, speed_modifier);
}

fn stop_navigation(mob: &dyn PathfinderMob) {
    mob.mob_base().navigation().lock().stop();
}

fn every(mob: &dyn PathfinderMob, interval: i32) -> bool {
    mob.tick_count().wrapping_add(mob.id()) % interval == 0
}

/// Run 5 blocks away from the attacker while hurt by a mob recently or burning.
pub struct PanicGoal {
    speed_modifier: f64,
}

impl PanicGoal {
    pub fn new(speed_modifier: f64) -> Self {
        Self { speed_modifier }
    }
}

impl Goal for PanicGoal {
    fn controls(&self) -> GoalControls {
        GoalControls::MOVE
    }
    fn can_use(&mut self, mob: &dyn PathfinderMob) -> bool {
        mob.last_hurt_by_mob().is_some() || mob.is_on_fire()
    }
    fn can_continue_to_use(&mut self, mob: &dyn PathfinderMob) -> bool {
        !navigation_is_done(mob)
    }
    fn start(&mut self, mob: &dyn PathfinderMob) {
        let pos = mob.position();
        let attacker = mob.last_hurt_by_mob().and_then(|id| mob.level()?.get_entity_by_id(id));
        let away = attacker
            .map(|a| pos - a.position())
            .filter(|d| d.horizontal_length() > 1e-3)
            .map(|d| d * (1.0 / d.horizontal_length()))
            .unwrap_or(Vec3::new(1.0, 0.0, 0.0));
        move_to(mob, pos + away * 5.0, self.speed_modifier);
        mob.log("panics");
    }
    fn stop(&mut self, mob: &dyn PathfinderMob) {
        stop_navigation(mob);
    }
}

/// While in love: walk to the nearest mate and breed after `BREED_DELAY` ticks next to it.
pub struct BreedGoal {
    speed_modifier: f64,
    partner: Option<SharedEntity>,
    love_time: i32,
}

const BREED_DELAY: i32 = 60;
const PARTNER_SEARCH_RADIUS: f64 = 8.0;

impl BreedGoal {
    pub fn new(speed_modifier: f64) -> Self {
        Self { speed_modifier, partner: None, love_time: 0 }
    }

    fn free_partner(mob: &dyn PathfinderMob) -> Option<SharedEntity> {
        let animal = mob.as_animal()?;
        let pos = mob.position();
        mob.level()?
            .entities_near(pos, PARTNER_SEARCH_RADIUS)
            .into_iter()
            .filter(|e| e.as_animal().is_some_and(|other| animal.can_mate(other)))
            .min_by(|a, b| a.position().distance(pos).total_cmp(&b.position().distance(pos)))
    }
}

impl Goal for BreedGoal {
    fn controls(&self) -> GoalControls {
        GoalControls::MOVE | GoalControls::LOOK
    }
    fn can_use(&mut self, mob: &dyn PathfinderMob) -> bool {
        if !mob.as_animal().is_some_and(|a| a.is_in_love()) {
            return false;
        }
        self.partner = Self::free_partner(mob);
        self.partner.is_some()
    }
    fn can_continue_to_use(&mut self, _mob: &dyn PathfinderMob) -> bool {
        self.partner
            .as_ref()
            .is_some_and(|p| p.is_alive() && p.as_animal().is_some_and(|a| a.is_in_love()))
            && self.love_time < BREED_DELAY
    }
    fn stop(&mut self, _mob: &dyn PathfinderMob) {
        self.partner = None;
        self.love_time = 0;
    }
    fn tick(&mut self, mob: &dyn PathfinderMob, _entity: &SharedEntity) {
        let Some(partner) = &self.partner else {
            return;
        };
        let target = partner.position();
        mob.mob_base().controls().lock().look_control = Some(mob.position().yaw_towards(target));
        move_to(mob, target, self.speed_modifier);
        self.love_time += 1;
        if self.love_time >= reduced_tick_delay(BREED_DELAY)
            && mob.position().distance(target) < 3.0
            && let (Some(animal), Some(mate), Some(world)) = (mob.as_animal(), partner.as_animal(), mob.level())
        {
            animal.spawn_child_from_breeding(&world, mate);
        }
    }
}

/// Babies follow the nearest adult of their type that is 3..16 blocks away.
pub struct FollowParentGoal {
    speed_modifier: f64,
    parent: Option<SharedEntity>,
    time_to_recalc_path: i32,
}

impl FollowParentGoal {
    pub fn new(speed_modifier: f64) -> Self {
        Self { speed_modifier, parent: None, time_to_recalc_path: 0 }
    }
}

impl Goal for FollowParentGoal {
    /// Vanilla sets no flags, so this runs alongside strolling.
    fn controls(&self) -> GoalControls {
        GoalControls::EMPTY
    }
    fn can_use(&mut self, mob: &dyn PathfinderMob) -> bool {
        if !mob.as_ageable_mob().is_some_and(|a| a.is_baby()) {
            return false;
        }
        let pos = mob.position();
        let Some(world) = mob.level() else {
            return false;
        };
        self.parent = world
            .entities_near(pos, 8.0)
            .into_iter()
            .filter(|e| e.entity_type() == mob.entity_type())
            .filter(|e| e.as_ageable_mob().is_some_and(|a| !a.is_baby()))
            .min_by(|a, b| a.position().distance(pos).total_cmp(&b.position().distance(pos)))
            .filter(|p| p.position().distance(pos) >= 3.0);
        self.parent.is_some()
    }
    fn can_continue_to_use(&mut self, mob: &dyn PathfinderMob) -> bool {
        let baby = mob.as_ageable_mob().is_some_and(|a| a.is_baby());
        baby && self.parent.as_ref().is_some_and(|p| {
            let d = p.position().distance(mob.position());
            p.is_alive() && (3.0..=16.0).contains(&d)
        })
    }
    fn start(&mut self, _mob: &dyn PathfinderMob) {
        self.time_to_recalc_path = 0;
    }
    fn stop(&mut self, _mob: &dyn PathfinderMob) {
        self.parent = None;
    }
    fn tick(&mut self, mob: &dyn PathfinderMob, _entity: &SharedEntity) {
        self.time_to_recalc_path -= 1;
        if self.time_to_recalc_path > 0 {
            return;
        }
        self.time_to_recalc_path = reduced_tick_delay(10);
        if let Some(parent) = &self.parent {
            move_to(mob, parent.position(), self.speed_modifier);
        }
    }
}

/// Every `interval` ticks walk `STROLL_DISTANCE` blocks, cycling east/south/west/north.
pub struct RandomStrollGoal {
    speed_modifier: f64,
    interval: i32,
    direction: usize,
    wanted: Vec3,
}

pub const STROLL_DISTANCE: f64 = 4.0;
const STROLL_DIRECTIONS: [Vec3; 4] =
    [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0)];

impl RandomStrollGoal {
    pub fn new(speed_modifier: f64, interval: i32) -> Self {
        Self { speed_modifier, interval, direction: 0, wanted: Vec3::ZERO }
    }
}

impl Goal for RandomStrollGoal {
    fn controls(&self) -> GoalControls {
        GoalControls::MOVE
    }
    fn can_use(&mut self, mob: &dyn PathfinderMob) -> bool {
        if !every(mob, self.interval) {
            return false;
        }
        let dir = STROLL_DIRECTIONS[self.direction % STROLL_DIRECTIONS.len()];
        self.direction += 1;
        self.wanted = mob.position() + dir * STROLL_DISTANCE;
        true
    }
    fn can_continue_to_use(&mut self, mob: &dyn PathfinderMob) -> bool {
        !navigation_is_done(mob)
    }
    fn start(&mut self, mob: &dyn PathfinderMob) {
        move_to(mob, self.wanted, self.speed_modifier);
    }
    fn stop(&mut self, mob: &dyn PathfinderMob) {
        stop_navigation(mob);
    }
}

/// Every 50 ticks turn the head 90 degrees (alternating sides) for 20 ticks.
pub struct RandomLookAroundGoal {
    target_yaw: f32,
    side: f32,
    look_time: i32,
}

impl RandomLookAroundGoal {
    pub fn new() -> Self {
        Self { target_yaw: 0.0, side: -1.0, look_time: 0 }
    }
}

impl Goal for RandomLookAroundGoal {
    fn controls(&self) -> GoalControls {
        GoalControls::MOVE | GoalControls::LOOK
    }
    fn can_use(&mut self, mob: &dyn PathfinderMob) -> bool {
        every(mob, 50)
    }
    fn can_continue_to_use(&mut self, _mob: &dyn PathfinderMob) -> bool {
        self.look_time >= 0
    }
    fn requires_update_every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, mob: &dyn PathfinderMob) {
        self.side = -self.side;
        self.target_yaw = mob.y_head_rot() + 90.0 * self.side;
        self.look_time = 20;
    }
    fn tick(&mut self, mob: &dyn PathfinderMob, _entity: &SharedEntity) {
        self.look_time -= 1;
        mob.mob_base().controls().lock().look_control = Some(self.target_yaw);
    }
}
