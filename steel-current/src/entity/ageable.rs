//! Steel: `ageable.rs`. Data: age. Tick step: age up until adult.

use crate::entity::Mob;
use crate::util::SyncMutex;

pub struct AgeableMobBase {
    age: SyncMutex<u32>,
}

impl AgeableMobBase {
    pub fn new(age: u32) -> Self {
        Self { age: SyncMutex::new(age) }
    }
}

pub trait AgeableMob: Mob {
    fn ageable_base(&self) -> &AgeableMobBase;
    /// Age at which the mob stops being a baby.
    fn adult_age(&self) -> u32;

    fn age(&self) -> u32 {
        *self.ageable_base().age.lock()
    }
    fn is_baby(&self) -> bool {
        self.age() < self.adult_age()
    }

    fn tick_ageable_mob(&self) {
        if !self.is_baby() {
            return;
        }
        *self.ageable_base().age.lock() += 1;
        if !self.is_baby() {
            self.log("grew up");
        }
    }
}
