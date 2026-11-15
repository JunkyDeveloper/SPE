//! Steel: `living_entity.rs` + `living_base`. Health, damage/invulnerability, death,
//! mob effects, head rotation and the `ai_step` -> `travel` movement step.

use std::sync::MutexGuard;

use crate::entity::{DamageSource, Entity, RemovalReason, SharedEntity};
use crate::util::{SyncMutex, Vec3};

/// Vanilla `LivingEntity.lastHurtByMob` memory.
pub const COMBAT_MEMORY_TICKS: i32 = 100;
const DEATH_TIME: i32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobEffect {
    Regeneration,
    Poison,
}

impl MobEffect {
    /// Vanilla `shouldApplyEffectTickThisTick` at amplifier 0.
    fn interval(self) -> i32 {
        match self {
            Self::Regeneration => 50,
            Self::Poison => 25,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MobEffectInstance {
    pub effect: MobEffect,
    pub duration: i32,
}

pub struct LivingState {
    pub health: f32,
    pub hurt_time: i32,
    pub invulnerable_time: i32,
    pub death_time: i32,
    pub dead: bool,
    pub last_hurt_by_mob: Option<i32>,
    pub last_hurt_by_mob_timestamp: i32,
    pub travel_input: Vec3,
    pub y_head_rot: f32,
    pub effects: Vec<MobEffectInstance>,
}

pub struct LivingEntityBase {
    max_health: f32,
    state: SyncMutex<LivingState>,
}

impl LivingEntityBase {
    pub fn new(max_health: f32) -> Self {
        Self {
            max_health,
            state: SyncMutex::new(LivingState {
                health: max_health,
                hurt_time: 0,
                invulnerable_time: 0,
                death_time: 0,
                dead: false,
                last_hurt_by_mob: None,
                last_hurt_by_mob_timestamp: 0,
                travel_input: Vec3::ZERO,
                y_head_rot: 0.0,
                effects: Vec::new(),
            }),
        }
    }
    pub fn state(&self) -> MutexGuard<'_, LivingState> {
        self.state.lock()
    }
}

pub trait LivingEntity: Entity {
    fn living_base(&self) -> &LivingEntityBase;

    // ---- data accessors ----
    fn get_health(&self) -> f32 {
        self.living_base().state().health
    }
    fn set_health(&self, health: f32) {
        let max = self.get_max_health();
        self.living_base().state().health = health.clamp(0.0, max);
    }
    fn get_max_health(&self) -> f32 {
        self.living_base().max_health
    }
    fn heal(&self, amount: f32) {
        if self.get_health() > 0.0 {
            self.set_health(self.get_health() + amount);
        }
    }
    fn is_dead_or_dying(&self) -> bool {
        self.get_health() <= 0.0
    }
    fn hurt_time(&self) -> i32 {
        self.living_base().state().hurt_time
    }
    fn last_hurt_by_mob(&self) -> Option<i32> {
        self.living_base().state().last_hurt_by_mob
    }
    fn y_head_rot(&self) -> f32 {
        self.living_base().state().y_head_rot
    }
    fn set_y_head_rot(&self, yaw: f32) {
        self.living_base().state().y_head_rot = yaw;
    }
    fn travel_input(&self) -> Vec3 {
        self.living_base().state().travel_input
    }
    fn set_travel_input(&self, input: Vec3) {
        self.living_base().state().travel_input = input;
    }
    fn add_effect(&self, effect: MobEffect, duration: i32) {
        let mut s = self.living_base().state();
        s.effects.retain(|e| e.effect != effect);
        s.effects.push(MobEffectInstance { effect, duration });
    }
    fn has_effect(&self, effect: MobEffect) -> bool {
        self.living_base().state().effects.iter().any(|e| e.effect == effect)
    }

    // ---- tick chain ----

    /// Steel: `base_tick_living_entity`. Base entity tick, hurt timer, combat memory expiry.
    fn base_tick_living_entity(&self) {
        self.entity_base_tick();
        let now = self.tick_count();
        let mut s = self.living_base().state();
        if s.hurt_time > 0 {
            s.hurt_time -= 1;
        }
        if s.last_hurt_by_mob.is_some() && now - s.last_hurt_by_mob_timestamp > COMBAT_MEMORY_TICKS {
            s.last_hurt_by_mob = None;
        }
    }

    /// Steel: vanilla `LivingEntity.tick`. The default `Entity::tick` dispatches here.
    fn tick_living_entity(&self, entity: &SharedEntity) {
        self.default_tick();
        {
            let mut s = self.living_base().state();
            if s.invulnerable_time > 0 {
                s.invulnerable_time -= 1;
            }
        }
        if self.is_dead_or_dying() {
            self.tick_death();
        }
        self.tick_mob_effects();
        if !self.is_removed() {
            self.ai_step(entity);
        }
    }

    fn tick_death(&self) {
        let death_time = {
            let mut s = self.living_base().state();
            s.death_time += 1;
            s.death_time
        };
        if death_time >= DEATH_TIME && !self.is_removed() {
            self.set_removed(RemovalReason::Killed);
        }
    }

    /// Applies each effect on its interval, then counts its duration down.
    fn tick_mob_effects(&self) {
        let due: Vec<MobEffect> = {
            let mut s = self.living_base().state();
            let due = s
                .effects
                .iter()
                .filter(|e| e.duration % e.effect.interval() == 0)
                .map(|e| e.effect)
                .collect();
            s.effects.iter_mut().for_each(|e| e.duration -= 1);
            s.effects.retain(|e| e.duration > 0);
            due
        };
        for effect in due {
            match effect {
                MobEffect::Regeneration => self.heal(1.0),
                MobEffect::Poison if self.get_health() > 1.0 => {
                    self.hurt(DamageSource::MAGIC, 1.0);
                }
                MobEffect::Poison => {}
            }
        }
    }

    // ---- damage ----

    /// Steel: `hurt_server`. Blocked while invulnerability is above 10 ticks.
    fn hurt_server(&self, source: DamageSource, amount: f32) -> bool {
        if self.is_removed() || self.is_dead_or_dying() {
            return false;
        }
        let now = self.tick_count();
        {
            let mut s = self.living_base().state();
            if s.invulnerable_time > 10 {
                return false;
            }
            s.invulnerable_time = 20;
            s.hurt_time = 10;
            if let Some(attacker) = source.attacker {
                s.last_hurt_by_mob = Some(attacker);
                s.last_hurt_by_mob_timestamp = now;
            }
        }
        self.actually_hurt(source, amount);
        if self.is_dead_or_dying() {
            self.die(source);
        }
        true
    }

    fn actually_hurt(&self, source: DamageSource, amount: f32) {
        self.set_health(self.get_health() - amount);
        self.log(&format!("hurt by {} for {amount}, health {}", source.kind, self.get_health()));
    }

    fn die(&self, source: DamageSource) {
        {
            let mut s = self.living_base().state();
            if s.dead {
                return;
            }
            s.dead = true;
        }
        self.log(&format!("died ({})", source.kind));
    }

    // ---- ai / movement ----

    fn is_immobile(&self) -> bool {
        self.is_dead_or_dying()
    }

    fn is_effective_ai(&self) -> bool {
        true
    }

    /// Server AI hook called from `default_ai_step`. Mobs forward to `Mob::mob_server_ai_step`.
    fn server_ai_step(&self, _entity: &SharedEntity) {}

    /// Steel: vanilla-shaped `LivingEntity.aiStep()`.
    fn default_ai_step(&self, entity: &SharedEntity) {
        if self.is_immobile() {
            self.set_travel_input(Vec3::ZERO);
        } else if self.is_effective_ai() {
            self.server_ai_step(entity);
        }
        self.travel(self.travel_input());
    }

    /// Override point: concrete types chain `Mob::mob_ai_step`, `AgeableMob::tick_ageable_mob`, ...
    fn ai_step(&self, entity: &SharedEntity) {
        self.default_ai_step(entity);
    }

    /// Steel: input -> velocity -> collision-aware move. Here the input is applied directly.
    fn travel(&self, input: Vec3) {
        self.set_position(self.position() + input);
    }
}
