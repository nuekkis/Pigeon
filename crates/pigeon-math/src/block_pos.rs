use serde::{Deserialize, Serialize};

/// Integer block coordinates in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn offset(self, dx: i32, dy: i32, dz: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            z: self.z + dz,
        }
    }

    pub fn chunk_pos(self) -> super::ChunkPos {
        super::ChunkPos::from_block(self.x, self.z)
    }

    pub fn section_y(self, section_height: u32) -> u32 {
        let shifted = self.y.div_euclid(section_height as i32);
        if shifted < 0 {
            0
        } else {
            shifted as u32
        }
    }

    pub fn section_relative(self, section_height: u32) -> (u32, u32, u32) {
        let sx = self.x.rem_euclid(16) as u32;
        let sy = self.y.rem_euclid(section_height as i32) as u32;
        let sz = self.z.rem_euclid(16) as u32;
        (sx, sy, sz)
    }
}

impl std::ops::Add<Self> for BlockPos {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}
