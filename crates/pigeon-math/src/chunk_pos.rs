use serde::{Deserialize, Serialize};

/// Coordinates of a 16×16 chunk column in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Compute a chunk coordinate from a block coordinate.
    pub fn from_block(block_x: i32, block_z: i32) -> Self {
        Self {
            x: block_x.div_euclid(16),
            z: block_z.div_euclid(16),
        }
    }

    pub fn block_origin(self) -> (i32, i32) {
        (self.x * 16, self.z * 16)
    }

    pub fn region_pos(self) -> (i32, i32) {
        (self.x.div_euclid(32), self.z.div_euclid(32))
    }

    pub fn region_local(self) -> (u32, u32) {
        (self.x.rem_euclid(32) as u32, self.z.rem_euclid(32) as u32)
    }

    pub fn square_distance(self, other: Self) -> i64 {
        let dx = (self.x - other.x) as i64;
        let dz = (self.z - other.z) as i64;
        dx * dx + dz * dz
    }

    pub fn chebyshev_distance(self, other: Self) -> i32 {
        let dx = (self.x - other.x).abs();
        let dz = (self.z - other.z).abs();
        dx.max(dz)
    }
}

impl std::ops::Add<Self> for ChunkPos {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
        }
    }
}

impl std::ops::Sub<Self> for ChunkPos {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            z: self.z - rhs.z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_from_block() {
        assert_eq!(ChunkPos::from_block(0, 0), ChunkPos::new(0, 0));
        assert_eq!(ChunkPos::from_block(15, 15), ChunkPos::new(0, 0));
        assert_eq!(ChunkPos::from_block(16, 16), ChunkPos::new(1, 1));
        assert_eq!(ChunkPos::from_block(-1, -1), ChunkPos::new(-1, -1));
        assert_eq!(ChunkPos::from_block(-16, -16), ChunkPos::new(-1, -1));
    }

    #[test]
    fn region_invariants() {
        let pos = ChunkPos::new(33, -5);
        assert_eq!(pos.region_pos(), (1, -1));
        assert_eq!(pos.region_local(), (1, 27));
    }
}
