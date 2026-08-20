/// Generic 2D position for pathfinding
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    /// Create a new position
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Calculate Manhattan distance to another position
    pub fn manhattan_distance(&self, other: &Pos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    /// Calculate Euclidean distance to another position
    pub fn euclidean_distance(&self, other: &Pos) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        (dx * dx + dy * dy).sqrt()
    }

    /// Calculate squared Euclidean distance (faster than euclidean_distance)
    pub fn euclidean_distance_squared(&self, other: &Pos) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        dx * dx + dy * dy
    }

    /// Get 4-way neighbors (N, S, E, W)
    pub fn neighbors_4way(&self) -> [Pos; 4] {
        [
            Pos::new(self.x + 1, self.y),
            Pos::new(self.x - 1, self.y),
            Pos::new(self.x, self.y + 1),
            Pos::new(self.x, self.y - 1),
        ]
    }

    /// Get 8-way neighbors (includes diagonals)
    pub fn neighbors_8way(&self) -> [Pos; 8] {
        [
            Pos::new(self.x + 1, self.y),
            Pos::new(self.x - 1, self.y),
            Pos::new(self.x, self.y + 1),
            Pos::new(self.x, self.y - 1),
            Pos::new(self.x + 1, self.y + 1),
            Pos::new(self.x + 1, self.y - 1),
            Pos::new(self.x - 1, self.y + 1),
            Pos::new(self.x - 1, self.y - 1),
        ]
    }
}

impl From<(i32, i32)> for Pos {
    fn from((x, y): (i32, i32)) -> Self {
        Self::new(x, y)
    }
}

impl From<Pos> for (i32, i32) {
    fn from(pos: Pos) -> Self {
        (pos.x, pos.y)
    }
}

/// A pathfinding grid with walkability and cost information
#[derive(Debug, Clone)]
pub struct PathfindingGrid {
    pub width: usize,
    pub height: usize,
    walkable: Vec<Vec<bool>>,
    cost: Vec<Vec<f32>>,
}

impl PathfindingGrid {
    /// Create a new pathfinding grid (all tiles walkable with cost 1.0)
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            walkable: vec![vec![true; width]; height],
            cost: vec![vec![1.0; width]; height],
        }
    }

    /// Check if a position is within bounds
    pub fn is_valid(&self, pos: Pos) -> bool {
        pos.x >= 0 && pos.y >= 0 && (pos.x as usize) < self.width && (pos.y as usize) < self.height
    }

    /// Check if a position is walkable
    pub fn is_walkable(&self, pos: Pos) -> bool {
        if !self.is_valid(pos) {
            return false;
        }
        self.walkable[pos.y as usize][pos.x as usize]
    }

    /// Get the cost of moving to a position
    pub fn get_cost(&self, pos: Pos) -> f32 {
        if !self.is_valid(pos) {
            return f32::INFINITY;
        }
        self.cost[pos.y as usize][pos.x as usize]
    }

    /// Set whether a position is walkable
    pub fn set_walkable(&mut self, pos: Pos, walkable: bool) {
        if self.is_valid(pos) {
            self.walkable[pos.y as usize][pos.x as usize] = walkable;
        }
    }

    /// Set the movement cost for a position
    pub fn set_cost(&mut self, pos: Pos, cost: f32) {
        if self.is_valid(pos) && cost.is_finite() && cost >= 0.0 {
            self.cost[pos.y as usize][pos.x as usize] = cost;
        }
    }

    /// Set a rectangular region as walkable/unwalkable
    pub fn set_region_walkable(&mut self, min: Pos, max: Pos, walkable: bool) {
        let min_x = min.x.max(0) as usize;
        let min_y = min.y.max(0) as usize;
        let max_x = (max.x.max(0) as usize).min(self.width);
        let max_y = (max.y.max(0) as usize).min(self.height);

        for y in min_y..max_y {
            for x in min_x..max_x {
                self.walkable[y][x] = walkable;
            }
        }
    }

    /// Set all tiles in a region to the same cost
    pub fn set_region_cost(&mut self, min: Pos, max: Pos, cost: f32) {
        let min_x = min.x.max(0) as usize;
        let min_y = min.y.max(0) as usize;
        let max_x = (max.x.max(0) as usize).min(self.width);
        let max_y = (max.y.max(0) as usize).min(self.height);

        if cost.is_finite() && cost >= 0.0 {
            for y in min_y..max_y {
                for x in min_x..max_x {
                    self.cost[y][x] = cost;
                }
            }
        }
    }

    /// Clear all obstacles (make everything walkable)
    pub fn clear(&mut self) {
        for row in &mut self.walkable {
            for cell in row {
                *cell = true;
            }
        }
        for row in &mut self.cost {
            for cell in row {
                *cell = 1.0;
            }
        }
    }
}

/// A pathfinding result with waypoints and cost
#[derive(Debug, Clone)]
pub struct Path {
    pub waypoints: Vec<Pos>,
    pub total_cost: f32,
}

impl Path {
    /// Create a new path
    pub fn new(waypoints: Vec<Pos>, total_cost: f32) -> Self {
        Self {
            waypoints,
            total_cost,
        }
    }

    /// Get the length of the path (number of waypoints)
    pub fn len(&self) -> usize {
        self.waypoints.len()
    }

    /// Check if path is empty
    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }

    /// Get the first waypoint (start position)
    pub fn start(&self) -> Option<Pos> {
        self.waypoints.first().copied()
    }

    /// Get the last waypoint (goal position)
    pub fn goal(&self) -> Option<Pos> {
        self.waypoints.last().copied()
    }

    /// Get the next waypoint after the current position
    pub fn next_after(&self, current: Pos) -> Option<Pos> {
        self.waypoints
            .iter()
            .position(|p| *p == current)
            .and_then(|idx| self.waypoints.get(idx + 1).copied())
    }
}

/// Heuristic function type for A*
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Heuristic {
    /// Manhattan distance (best for 4-way movement)
    Manhattan,
    /// Euclidean distance (best for 8-way movement)
    Euclidean,
}

impl Heuristic {
    pub(crate) fn estimate(&self, from: Pos, to: Pos) -> f32 {
        match self {
            Heuristic::Manhattan => from.manhattan_distance(&to) as f32,
            Heuristic::Euclidean => from.euclidean_distance(&to),
        }
    }
}
