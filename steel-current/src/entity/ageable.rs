//! Steel: `ageable.rs`. Negative age = baby counting up to 0; positive age = breeding
//! cooldown counting down to 0.

use std::sync::Arc;

use crate::entity::{Mob, SharedEntity};
use crate::util::SyncMutex;
use crate::world::World;

pub const BABY_START_AGE: i32 = -60; // vanilla -24000

pub struct AgeableMobBase {
    age: SyncMutex<i32>,
}

impl AgeableMobBase {
    pub fn new(age: i32) -> Self {
        Self { age: SyncMutex::new(age) }
    }
}

pub trait AgeableMob: Mob {
    fn ageable_base(&self) -> &AgeableMobBase;

    fn get_age(&self) -> i32 {
        *self.ageable_base().age.lock()
    }
    fn set_age(&self, age: i32) {
        let was_baby = self.is_baby();
        *self.ageable_base().age.lock() = age;
        if was_baby != self.is_baby() {
            self.age_boundary_changed(!was_baby);
        }
    }
    fn is_baby(&self) -> bool {
        self.get_age() < 0
    }
    fn can_age_up(&self) -> bool {
        self.get_age() < 0
    }
    fn get_baby_start_age(&self) -> i32 {
        BABY_START_AGE
    }
    /// Steel refreshes dimensions here.
    fn age_boundary_changed(&self, baby: bool) {
        if !baby {
            self.log("grew up");
        }
    }

    /// Vanilla `AgeableMob.getBreedOffspring`. The caller adds it to the world.
    fn get_breed_offspring(&self, _world: &Arc<World>) -> Option<SharedEntity> {
        None
    }

    fn tick_ageable_mob(&self) {
        if !self.is_alive() {
            return;
        }
        let age = self.get_age();
        if self.can_age_up() {
            self.set_age(age + 1);
        } else if age > 0 {
            self.set_age(age - 1);
        }
    }
}
