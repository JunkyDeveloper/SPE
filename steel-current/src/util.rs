//! Std-only helpers. Steel: `glam::DVec3`, `steel_utils::locks::SyncMutex`.

use std::ops::{Add, Mul};
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
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, k: f64) -> Vec3 {
        Vec3::new(self.x * k, self.y * k, self.z * k)
    }
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
