use crate::Vec3;
use serde::{Deserialize, Serialize};

/// An axis-aligned bounding box, used for entity collision and hit-tests.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_corners(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> Self {
        Self {
            min: Vec3::new(x1.min(x2), y1.min(y2), z1.min(z2)),
            max: Vec3::new(x1.max(x2), y1.max(y2), z1.max(z2)),
        }
    }

    pub fn size(self) -> Vec3 {
        self.max.sub(self.min)
    }

    pub fn center(self) -> Vec3 {
        Vec3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }

    pub fn expand(self, amount: f64) -> Self {
        Self {
            min: Vec3::new(
                self.min.x - amount,
                self.min.y - amount,
                self.min.z - amount,
            ),
            max: Vec3::new(
                self.max.x + amount,
                self.max.y + amount,
                self.max.z + amount,
            ),
        }
    }

    pub fn offset(self, delta: Vec3) -> Self {
        Self {
            min: self.min.add(delta),
            max: self.max.add(delta),
        }
    }

    pub fn intersects(self, other: Self) -> bool {
        self.max.x > other.min.x
            && self.min.x < other.max.x
            && self.max.y > other.min.y
            && self.min.y < other.max.y
            && self.max.z > other.min.z
            && self.min.z < other.max.z
    }

    pub fn contains_point(self, p: Vec3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }
}
