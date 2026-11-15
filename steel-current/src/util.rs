//! Std-only helpers. Steel: `glam::DVec3`, `steel_utils::locks::SyncMutex`, `ItemStack`, `BlockPos`.

use std::ops::{Add, Mul, Sub};
use std::sync::{Mutex, MutexGuard};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3::new(0.0, 0.0, 0.0);
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub fn horizontal_length(self) -> f64 {
        (self.x * self.x + self.z * self.z).sqrt()
    }
    pub fn distance(self, o: Vec3) -> f64 {
        let d = self - o;
        (d.x * d.x + d.y * d.y + d.z * d.z).sqrt()
    }
    /// Minecraft yaw (degrees) looking from `self` towards `target`.
    pub fn yaw_towards(self, target: Vec3) -> f32 {
        let d = target - self;
        (d.z.atan2(d.x).to_degrees() - 90.0) as f32
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, k: f64) -> Vec3 {
        Vec3::new(self.x * k, self.y * k, self.z * k)
    }
}

pub type BlockPos = (i32, i32, i32);

/// Steel: `steel_registry::item_stack::ItemStack`, reduced to an id + count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemStack {
    pub item: &'static str,
    pub count: u32,
}

impl ItemStack {
    pub const EMPTY: ItemStack = ItemStack::new("air", 0);
    pub const fn new(item: &'static str, count: u32) -> Self {
        Self { item, count }
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn is(&self, item: &str) -> bool {
        !self.is_empty() && self.item == item
    }
}

/// Steel: `behavior::InteractionResult`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionResult {
    Success,
    Pass,
}

/// `Mutex` whose `lock()` doesn't return a `Result`.
#[derive(Default)]
pub struct SyncMutex<T>(Mutex<T>);

impl<T> SyncMutex<T> {
    pub const fn new(value: T) -> Self {
        Self(Mutex::new(value))
    }
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.0.lock().expect("mutex poisoned")
    }
}
