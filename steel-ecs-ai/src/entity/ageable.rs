//! Steel: `ageable.rs`. Part: age. Stage: age up until adult.

use super::{Cx, Entity};

pub struct Ageable {
    pub age: u32,
    /// Age at which the mob stops being a baby.
    pub adult_age: u32,
}

impl Ageable {
    pub fn new(age: u32, adult_age: u32) -> Self {
        Self { age, adult_age }
    }

    pub fn is_baby(&self) -> bool {
        self.age < self.adult_age
    }
}

pub fn age_up(e: &mut Entity, _cx: &mut Cx) {
    let ageable = e.ageable_mut();
    if !ageable.is_baby() {
        return;
    }
    ageable.age += 1;
    if !ageable.is_baby() {
        e.log("grew up");
    }
}
