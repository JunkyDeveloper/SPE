//! Steel: `living_entity.rs` / `living_base`. Part: health. Stage: regenerate.

use super::{Cx, Entity};

pub struct Living {
    pub health: i32,
    pub max_health: i32,
}

impl Living {
    pub fn new(health: i32, max_health: i32) -> Self {
        Self { health, max_health }
    }
}

/// +1 health every 20 ticks until full.
pub fn regenerate(e: &mut Entity, _cx: &mut Cx) {
    if !e.base.tick_count.is_multiple_of(20) {
        return;
    }
    let living = e.living_mut();
    if living.health >= living.max_health {
        return;
    }
    living.health += 1;
    let (health, max) = (living.health, living.max_health);
    e.log(format_args!("health {health}/{max}"));
}
