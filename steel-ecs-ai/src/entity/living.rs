//! Steel: `living_entity.rs` / `living_base`. Data: health. Tick step: regenerate.

use crate::ecs::World;
use crate::entity::EntityHeader;
use crate::log::Logs;

pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    pub fn new(current: i32, max: i32) -> Self {
        Self { current, max }
    }
}

/// +1 health every 20 ticks until full.
pub fn tick_regeneration(world: &mut World) {
    let header = EntityHeader::new(world);
    let mut health = world.write::<Health>();
    let mut logs = world.res_mut::<Logs>();
    for (entity, h) in health.iter_mut() {
        if !header.is_active(entity)
            || !header.tick_count(entity).is_multiple_of(20)
            || h.current >= h.max
        {
            continue;
        }
        h.current += 1;
        header.log(
            &mut logs,
            entity,
            &format!("health {}/{}", h.current, h.max),
        );
    }
}
