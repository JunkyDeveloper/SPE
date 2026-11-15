//! Steel: `entity/entities/mobs/passive/cow/mod.rs`. Full chain: all five bases.

use std::sync::Arc;

use crate::entity::*;
use crate::util::{InteractionResult, ItemStack, Vec3};
use crate::world::World;

pub struct CowEntity {
    base: EntityBase,
    living_base: LivingEntityBase,
    mob_base: MobBase,
    ageable_base: AgeableMobBase,
    animal_base: AnimalBase,
}

impl CowEntity {
    /// Negative `age` means baby (see `BABY_START_AGE`).
    pub fn new(id: i32, position: Vec3, age: i32) -> Self {
        let mob_base = MobBase::new();
        {
            // Same priorities as Steel's cow.
            let mut goals = mob_base.goal_selector().lock();
            goals.add_goal(1, PanicGoal::new(2.0));
            goals.add_goal(2, BreedGoal::new(1.0));
            goals.add_goal(4, FollowParentGoal::new(1.25));
            goals.add_goal(5, RandomStrollGoal::new(1.0, 60));
            goals.add_goal(7, RandomLookAroundGoal::new());
        }
        Self {
            base: EntityBase::new(id, "cow", position),
            living_base: LivingEntityBase::new(10.0),
            mob_base,
            ageable_base: AgeableMobBase::new(age),
            animal_base: AnimalBase::new(),
        }
    }
}

impl Entity for CowEntity {
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

impl LivingEntity for CowEntity {
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
    }
}

impl Mob for CowEntity {
    fn mob_base(&self) -> &MobBase {
        &self.mob_base
    }
    fn movement_speed(&self) -> f64 {
        0.1
    }
    fn ambient_sound(&self) -> Option<&'static str> {
        Some("moo")
    }
    fn tick_goal_selectors(&self, entity: &SharedEntity) {
        PathfinderMob::tick_pathfinder_goal_selectors(self, entity);
    }
    fn custom_server_ai_step(&self) {
        Animal::custom_server_ai_step_animal(self);
    }
    fn mob_interact(&self, item: ItemStack) -> InteractionResult {
        if item.is("bucket") && !self.is_baby() {
            self.log("milked");
            return InteractionResult::Success;
        }
        Animal::mob_interact_animal(self, item)
    }
}

impl PathfinderMob for CowEntity {}

impl AgeableMob for CowEntity {
    fn ageable_base(&self) -> &AgeableMobBase {
        &self.ageable_base
    }
    fn get_breed_offspring(&self, world: &Arc<World>) -> Option<SharedEntity> {
        Some(Arc::new(CowEntity::new(world.next_entity_id(), self.position(), 0)))
    }
}

impl Animal for CowEntity {
    fn animal_base(&self) -> &AnimalBase {
        &self.animal_base
    }
    fn is_food(&self, item: &ItemStack) -> bool {
        item.is("wheat")
    }
}
