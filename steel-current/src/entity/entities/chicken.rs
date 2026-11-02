//! Steel: `entity/entities/mobs/passive/chicken/mod.rs`. Full chain: all five bases.

use crate::entity::*;
use crate::util::Vec3;

pub struct ChickenEntity {
    base: EntityBase,
    living_base: LivingEntityBase,
    mob_base: MobBase,
    ageable_base: AgeableMobBase,
    animal_base: AnimalBase,
}

impl ChickenEntity {
    /// `age` below 30 means baby.
    pub fn new(id: u32, position: Vec3, age: u32) -> Self {
        let mob_base = MobBase::new();
        mob_base.add_goal(1, Box::new(HopGoal::new())); // chicken only
        mob_base.add_goal(5, Box::new(WanderGoal::new())); // shared
        mob_base.add_goal(7, Box::new(LookAroundGoal::new())); // shared
        Self {
            base: EntityBase::new(id, "chicken", position),
            living_base: LivingEntityBase::new(2, 4),
            mob_base,
            ageable_base: AgeableMobBase::new(age),
            animal_base: AnimalBase::new(),
        }
    }
}

impl Entity for ChickenEntity {
    fn base(&self) -> &EntityBase {
        &self.base
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
    fn ai_step(&self) {
        Mob::mob_ai_step(self);
        AgeableMob::tick_ageable_mob(self);
        Animal::tick_animal(self);
    }
}

impl Mob for ChickenEntity {
    fn mob_base(&self) -> &MobBase {
        &self.mob_base
    }
    fn move_speed(&self) -> f64 {
        0.25
    }
}

impl AgeableMob for ChickenEntity {
    fn ageable_base(&self) -> &AgeableMobBase {
        &self.ageable_base
    }
    fn adult_age(&self) -> u32 {
        30
    }
}

impl Animal for ChickenEntity {
    fn animal_base(&self) -> &AnimalBase {
        &self.animal_base
    }
    fn hunger_limit(&self) -> u32 {
        25
    }
}
