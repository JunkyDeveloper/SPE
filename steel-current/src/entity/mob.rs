//! Steel: `mob/mod.rs` + `mob/pathfinder.rs`. Goal selector, path navigation,
//! move/look controls and the ambient sound timer.

use crate::entity::{GoalSelector, LivingEntity, SharedEntity};
use crate::util::{InteractionResult, ItemStack, SyncMutex, Vec3};

/// Navigation is done once the mob is this close (horizontally) to its target.
const REACH_DISTANCE: f64 = 0.5;
/// Max head turn per tick (vanilla `LookControl` default).
const MAX_HEAD_TURN: f32 = 10.0;

/// Steel: `ai/navigation.rs`. No pathfinding: the path is a straight line to the target.
#[derive(Default)]
pub struct PathNavigation {
    target: Option<Vec3>,
    speed_modifier: f64,
}

impl PathNavigation {
    pub fn move_to(&mut self, target: Vec3, speed_modifier: f64) {
        self.target = Some(target);
        self.speed_modifier = speed_modifier;
    }
    pub fn stop(&mut self) {
        self.target = None;
    }
    pub fn is_done(&self) -> bool {
        self.target.is_none()
    }
    /// Returns the next wanted position for the move control, or `None` once arrived.
    pub fn tick(&mut self, position: Vec3) -> Option<(Vec3, f64)> {
        let target = self.target?;
        if (target - position).horizontal_length() < REACH_DISTANCE {
            self.target = None;
            return None;
        }
        Some((target, self.speed_modifier))
    }
}

/// Steel: `MoveControl` / `LookControl`. Both are one-shot: whoever wants them
/// must set them again every tick (vanilla resets the operation after each tick).
#[derive(Default)]
pub struct MobControls {
    pub move_control: Option<(Vec3, f64)>,
    pub look_control: Option<f32>,
}

pub struct MobBase {
    goal_selector: SyncMutex<GoalSelector>,
    navigation: SyncMutex<PathNavigation>,
    controls: SyncMutex<MobControls>,
    ambient_sound_time: SyncMutex<i32>,
}

impl MobBase {
    pub fn new() -> Self {
        Self {
            goal_selector: SyncMutex::new(GoalSelector::default()),
            navigation: SyncMutex::default(),
            controls: SyncMutex::default(),
            ambient_sound_time: SyncMutex::new(0),
        }
    }
    pub fn goal_selector(&self) -> &SyncMutex<GoalSelector> {
        &self.goal_selector
    }
    pub fn navigation(&self) -> &SyncMutex<PathNavigation> {
        &self.navigation
    }
    pub fn controls(&self) -> &SyncMutex<MobControls> {
        &self.controls
    }
}

pub trait Mob: LivingEntity {
    fn mob_base(&self) -> &MobBase;
    /// Steel: `MOVEMENT_SPEED` attribute. Blocks per tick of input at speed modifier 1.0.
    fn movement_speed(&self) -> f64;

    fn ambient_sound(&self) -> Option<&'static str> {
        None
    }
    fn ambient_sound_interval(&self) -> i32 {
        80
    }
    fn play_ambient_sound(&self) {
        if let Some(sound) = self.ambient_sound() {
            self.log(sound);
        }
    }

    /// Steel: `Mob.setBaby`; ageable mobs derive it from their start age.
    fn set_baby(&self, baby: bool) {
        if let Some(ageable) = self.as_ageable_mob() {
            ageable.set_age(if baby { ageable.get_baby_start_age() } else { 0 });
        }
    }

    /// Steel: `mob_interact`. Players are out of scope, so the held item is passed directly.
    fn mob_interact(&self, _item: ItemStack) -> InteractionResult {
        InteractionResult::Pass
    }

    // ---- base tick ----

    /// Shared living base tick, then the mob-owned part.
    fn base_tick_mob(&self) {
        self.base_tick_living_entity();
        self.mob_base_tick();
    }

    /// Vanilla rolls `random(1000) < ambientSoundTime`; here it simply fires every interval.
    fn mob_base_tick(&self) {
        if !self.is_alive() || self.is_dead_or_dying() {
            return;
        }
        let play = {
            let mut t = self.mob_base().ambient_sound_time.lock();
            *t += 1;
            if *t >= self.ambient_sound_interval() {
                *t = 0;
                true
            } else {
                false
            }
        };
        if play {
            self.play_ambient_sound();
        }
    }

    // ---- ai step ----

    fn mob_server_ai_step(&self, entity: &SharedEntity) {
        self.tick_goal_selectors(entity);
        self.tick_path_navigation();
        self.custom_server_ai_step();
        self.tick_move_control();
        self.tick_look_control();
    }

    /// Vanilla also burns undead in daylight and picks up loot here.
    fn mob_ai_step(&self, entity: &SharedEntity) {
        self.default_ai_step(entity);
    }

    /// Override point: pathfinder mobs forward to `PathfinderMob::tick_pathfinder_goal_selectors`.
    fn tick_goal_selectors(&self, _entity: &SharedEntity) {}

    fn custom_server_ai_step(&self) {}

    fn tick_path_navigation(&self) {
        let pos = self.position();
        let wanted = self.mob_base().navigation().lock().tick(pos);
        if wanted.is_some() {
            self.mob_base().controls().lock().move_control = wanted;
        }
    }

    /// Turns the wanted position into horizontal travel input.
    fn tick_move_control(&self) {
        let wanted = self.mob_base().controls().lock().move_control.take();
        let input = wanted
            .map(|(target, speed_modifier)| {
                let d = target - self.position();
                let dist = d.horizontal_length();
                if dist < 1e-6 {
                    return Vec3::ZERO;
                }
                let speed = (self.movement_speed() * speed_modifier).min(dist);
                Vec3::new(d.x / dist * speed, 0.0, d.z / dist * speed)
            })
            .unwrap_or(Vec3::ZERO);
        self.set_travel_input(input);
    }

    /// Rotates the head towards the wanted yaw, at most `MAX_HEAD_TURN` degrees per tick.
    fn tick_look_control(&self) {
        let Some(wanted) = self.mob_base().controls().lock().look_control.take() else {
            return;
        };
        let head = self.y_head_rot();
        let diff = (wanted - head + 540.0).rem_euclid(360.0) - 180.0;
        self.set_y_head_rot(head + diff.clamp(-MAX_HEAD_TURN, MAX_HEAD_TURN));
    }
}

/// Steel: `mob/pathfinder.rs`. Goals run against `&dyn PathfinderMob`.
pub trait PathfinderMob: Mob {
    /// Full selector tick every other tick (id-staggered); in between only goals that
    /// `requires_update_every_tick` are ticked.
    fn tick_pathfinder_goal_selectors(&self, entity: &SharedEntity)
    where
        Self: Sized,
    {
        let tick = self.tick_count();
        let mut goal_selector = self.mob_base().goal_selector().lock();
        if tick.wrapping_add(self.id()) % 2 != 0 && tick > 1 {
            goal_selector.tick_running_goals(self, entity, false);
        } else {
            goal_selector.tick(self, entity);
        }
    }
}
