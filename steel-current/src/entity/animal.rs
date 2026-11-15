//! Steel: `animal.rs`. Love mode (fed food), mating check, spawning a baby.

use std::sync::Arc;

use crate::entity::AgeableMob;
use crate::util::{InteractionResult, ItemStack, SyncMutex};
use crate::world::World;

pub const IN_LOVE_TIME: i32 = 100; // vanilla 600
pub const BREED_COOLDOWN: i32 = 100; // vanilla 6000

pub struct AnimalBase {
    in_love_time: SyncMutex<i32>,
}

impl AnimalBase {
    pub fn new() -> Self {
        Self { in_love_time: SyncMutex::new(0) }
    }
}

pub trait Animal: AgeableMob {
    fn animal_base(&self) -> &AnimalBase;

    fn in_love_time(&self) -> i32 {
        *self.animal_base().in_love_time.lock()
    }
    fn set_in_love_time(&self, ticks: i32) {
        *self.animal_base().in_love_time.lock() = ticks;
    }
    fn is_in_love(&self) -> bool {
        self.in_love_time() > 0
    }
    fn can_fall_in_love(&self) -> bool {
        self.in_love_time() <= 0
    }
    fn set_in_love(&self) {
        self.set_in_love_time(IN_LOVE_TIME);
        self.log("is in love");
    }
    fn reset_love(&self) {
        self.set_in_love_time(0);
    }
    /// Distinct entities of the same type, both in love.
    fn can_mate(&self, partner: &dyn Animal) -> bool {
        self.id() != partner.id()
            && self.entity_type() == partner.entity_type()
            && self.is_in_love()
            && partner.is_in_love()
    }
    fn is_food(&self, _item: &ItemStack) -> bool {
        false
    }

    /// Food makes adults fall in love and speeds up babies by 10% of their remaining age.
    fn mob_interact_animal(&self, item: ItemStack) -> InteractionResult {
        if !self.is_food(&item) {
            return InteractionResult::Pass;
        }
        let age = self.get_age();
        if age == 0 && self.can_fall_in_love() {
            self.set_in_love();
            return InteractionResult::Success;
        }
        if self.is_baby() {
            self.set_age(age - age / 10);
            return InteractionResult::Success;
        }
        InteractionResult::Pass
    }

    fn spawn_child_from_breeding(&self, world: &Arc<World>, partner: &dyn Animal) {
        let Some(child) = self.get_breed_offspring(world) else {
            return;
        };
        if let Some(mob) = child.as_mob() {
            mob.set_baby(true);
        }
        child.set_position(self.position());
        world.add_entity(child);
        self.set_age(BREED_COOLDOWN);
        partner.set_age(BREED_COOLDOWN);
        self.reset_love();
        partner.reset_love();
        self.log(&format!("bred with #{}", partner.id()));
    }

    fn tick_animal_love(&self) {
        if self.get_age() != 0 {
            self.reset_love();
            return;
        }
        let mut t = self.animal_base().in_love_time.lock();
        if *t > 0 {
            *t -= 1;
        }
    }

    fn custom_server_ai_step_animal(&self) {
        if self.get_age() != 0 {
            self.reset_love();
        }
    }
}
