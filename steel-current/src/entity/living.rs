//! Steel: `living_entity.rs` / `living_base`. Data: health. Tick step: regenerate.

use crate::entity::Entity;
use crate::util::SyncMutex;

pub struct LivingEntityBase {
    health: SyncMutex<i32>,
    max_health: i32,
}

impl LivingEntityBase {
    pub fn new(health: i32, max_health: i32) -> Self {
        Self { health: SyncMutex::new(health), max_health }
    }
}

pub trait LivingEntity: Entity {
    fn living_base(&self) -> &LivingEntityBase;

    fn health(&self) -> i32 {
        *self.living_base().health.lock()
    }
    fn max_health(&self) -> i32 {
        self.living_base().max_health
    }

    /// Steel: `LivingEntity.tick`.
    fn tick_living_entity(&self) {
        self.default_tick();
        self.tick_regeneration();
        self.ai_step();
    }

    /// +1 health every 20 ticks until full.
    fn tick_regeneration(&self) {
        if self.tick_count() % 20 != 0 {
            return;
        }
        let max = self.max_health();
        let new = {
            let mut h = self.living_base().health.lock();
            if *h >= max {
                return;
            }
            *h += 1;
            *h
        };
        self.log(&format!("health {new}/{max}"));
    }

    /// Override point: concrete types chain `Mob::mob_ai_step`, `AgeableMob::tick_ageable_mob`, ...
    fn ai_step(&self) {}
}
