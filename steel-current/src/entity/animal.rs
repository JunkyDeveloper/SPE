//! Steel: `animal.rs`. Data: hunger. Tick step: get hungrier up to the limit, log once when hungry.
//! Something else (a goal) has to reset it.

use crate::entity::AgeableMob;
use crate::util::SyncMutex;

pub struct AnimalBase {
    hunger: SyncMutex<u32>,
}

impl AnimalBase {
    pub fn new() -> Self {
        Self { hunger: SyncMutex::new(0) }
    }
}

pub trait Animal: AgeableMob {
    fn animal_base(&self) -> &AnimalBase;
    fn hunger_limit(&self) -> u32;

    fn hunger(&self) -> u32 {
        *self.animal_base().hunger.lock()
    }
    fn is_hungry(&self) -> bool {
        self.hunger() >= self.hunger_limit()
    }
    fn reset_hunger(&self) {
        *self.animal_base().hunger.lock() = 0;
    }

    fn tick_animal(&self) {
        let became_hungry = {
            let mut h = self.animal_base().hunger.lock();
            if *h < self.hunger_limit() {
                *h += 1;
                *h == self.hunger_limit()
            } else {
                false
            }
        };
        if became_hungry {
            self.log("is hungry");
        }
    }
}
