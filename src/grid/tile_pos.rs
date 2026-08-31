//! Tile coordinates and neighbour queries.

use serde::{Deserialize, Serialize};

/// A position in tile coordinates
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

impl TilePos {
    /// Create a new tile position
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Calculate Manhattan distance to another position, saturating at `i32::MAX`.
    pub fn manhattan_distance(&self, other: &TilePos) -> i32 {
        self.x
            .abs_diff(other.x)
            .saturating_add(self.y.abs_diff(other.y))
            .min(i32::MAX as u32) as i32
    }

    /// Calculate Euclidean distance to another position
    pub fn distance_to(&self, other: &TilePos) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        (dx * dx + dy * dy).sqrt()
    }

    /// Get 4-way neighbors (N, S, E, W)
    pub fn neighbors_4way(&self) -> [TilePos; 4] {
        [
            TilePos::new(self.x, self.y - 1),
            TilePos::new(self.x + 1, self.y),
            TilePos::new(self.x, self.y + 1),
            TilePos::new(self.x - 1, self.y),
        ]
    }

    /// Get 8-way neighbors (includes diagonals)
    pub fn neighbors_8way(&self) -> [TilePos; 8] {
        [
            TilePos::new(self.x, self.y - 1),
            TilePos::new(self.x + 1, self.y - 1),
            TilePos::new(self.x + 1, self.y),
            TilePos::new(self.x + 1, self.y + 1),
            TilePos::new(self.x, self.y + 1),
            TilePos::new(self.x - 1, self.y + 1),
            TilePos::new(self.x - 1, self.y),
            TilePos::new(self.x - 1, self.y - 1),
        ]
    }

    /// Convert to (f32, f32) tuple
    pub fn to_f32(&self) -> (f32, f32) {
        (self.x as f32, self.y as f32)
    }

    /// Check if this position is inside a width/height grid.
    pub fn in_bounds(&self, width: usize, height: usize) -> bool {
        self.x >= 0 && self.y >= 0 && (self.x as usize) < width && (self.y as usize) < height
    }

    /// Convert this position to a flat vector index.
    pub fn to_index(&self, width: usize) -> Option<usize> {
        if self.x < 0 || self.y < 0 {
            return None;
        }
        Some(self.y as usize * width + self.x as usize)
    }

    /// Convert a flat vector index to a tile position.
    pub fn from_index(index: usize, width: usize) -> Self {
        Self::new((index % width) as i32, (index / width) as i32)
    }
}

impl From<(i32, i32)> for TilePos {
    fn from((x, y): (i32, i32)) -> Self {
        Self::new(x, y)
    }
}

impl From<TilePos> for (i32, i32) {
    fn from(pos: TilePos) -> Self {
        (pos.x, pos.y)
    }
}
