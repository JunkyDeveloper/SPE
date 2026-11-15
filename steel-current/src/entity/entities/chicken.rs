//! Steel: `entity/entities/mobs/passive/chicken/mod.rs`. Full chain plus the chicken-only
//! `ai_step` extra: egg laying.

use std::sync::Arc;

use crate::entity::entities::item::ItemEntity;
use crate::entity::*;
use crate::util::{ItemStack, SyncMutex, Vec3};
use crate::world::World;

pub const EGG_LAY_TIME: i32 = 120; // vanilla random 6000..12000

pub struct ChickenEntity {
    base: EntityBase,
    living_base: LivingEntityBase,
    mob_base: MobBase,
    ageable_base: AgeableMobBase,
    animal_base: AnimalBase,
    egg_time: SyncMutex<i32>,
}

impl ChickenEntity {
    /// Negative `age` means baby (see `BABY_START_AGE`).
    pub fn new(id: i32, position: Vec3, age: i32) -> Self {
        let mob_base = MobBase::new();
        {
            let mut goals = mob_base.goal_selector().lock();
            goals.add_goal(1, PanicGoal::new(1.4));
            goals.add_goal(2, BreedGoal::new(1.0));
            goals.add_goal(4, FollowParentGoal::new(1.1));
            goals.add_goal(5, RandomStrollGoal::new(1.0, 60));
            goals.add_goal(7, RandomLookAroundGoal::new());
        }
        Self {
            base: EntityBase::new(id, "chicken", position),
            living_base: LivingEntityBase::new(4.0),
            mob_base,
            ageable_base: AgeableMobBase::new(age),
            animal_base: AnimalBase::new(),
            egg_time: SyncMutex::new(EGG_LAY_TIME),
        }
    }

    fn tick_egg_laying(&self) {
        if !self.is_alive() || self.is_baby() {
            return;
        }
        let lay = {
            let mut t = self.egg_time.lock();
            *t -= 1;
            if *t <= 0 {
                *t = EGG_LAY_TIME;
                true
            } else {
                false
            }
        };
        if let (true, Some(world)) = (lay, self.level()) {
            let pos = self.position();
            world.spawn(|id| ItemEntity::new(id, pos, ItemStack::new("egg", 1)));
            self.log("laid an egg");
        }
    }
}

impl Entity for ChickenEntity {
    fn base(&self) -> &EntityBase {
        &self.base
    }
    fn base_tick(&self) {
        Mob::base_tick_mob(self);
    }
    fn as_living_entity(&self) -> Option<&dyn LivingEntity> {
        Some(self)
    }
    fn as_mob(&self) -> Option<&dyn Mob> {
        Some(self)
    }
    fn as_ageable_mob(&self) -> Option<&dyn AgeableMob> {
        Some(self)
    }
    fn as_animal(&self) -> Option<&dyn Animal> {
        Some(self)
    }
}

impl LivingEntity for ChickenEntity {
    fn living_base(&self) -> &LivingEntityBase {
        &self.living_base
    }
    fn server_ai_step(&self, entity: &SharedEntity) {
        Mob::mob_server_ai_step(self, entity);
    }
    fn ai_step(&self, entity: &SharedEntity) {
        Mob::mob_ai_step(self, entity);
        AgeableMob::tick_ageable_mob(self);
        Animal::tick_animal_love(self);
        self.tick_egg_laying();
    }
}

impl Mob for ChickenEntity {
    fn mob_base(&self) -> &MobBase {
        &self.mob_base
    }
    fn movement_speed(&self) -> f64 {
        0.125
    }
    fn ambient_sound(&self) -> Option<&'static str> {
        Some("cluck")
    }
    fn tick_goal_selectors(&self, entity: &SharedEntity) {
        PathfinderMob::tick_pathfinder_goal_selectors(self, entity);
    }
    fn custom_server_ai_step(&self) {
        Animal::custom_server_ai_step_animal(self);
    }
    fn mob_interact(&self, item: ItemStack) -> crate::util::InteractionResult {
        Animal::mob_interact_animal(self, item)
    }
}

impl PathfinderMob for ChickenEntity {}

impl AgeableMob for ChickenEntity {
    fn ageable_base(&self) -> &AgeableMobBase {
        &self.ageable_base
    }
    fn get_breed_offspring(&self, world: &Arc<World>) -> Option<SharedEntity> {
        Some(Arc::new(ChickenEntity::new(world.next_entity_id(), self.position(), 0)))
    }
}

impl Animal for ChickenEntity {
    fn animal_base(&self) -> &AnimalBase {
        &self.animal_base
    }
    fn is_food(&self, item: &ItemStack) -> bool {
        item.is("wheat_seeds")
    }
}
