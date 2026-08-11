//! A* Pathfinding module
//!
//! Provides generic A* pathfinding for grid-based games with support for:
//! - 4-way and 8-way movement
//! - Variable movement costs
//! - Path caching with invalidation
//! - Multiple heuristic options
//!
//! # Example
//! ```
//! use macroquad_toolkit::pathfinding::{PathfindingGrid, Pos, find_path, Heuristic};
//!
//! let mut grid = PathfindingGrid::new(10, 10);
//! grid.set_walkable(Pos::new(5, 5), false); // Add an obstacle
//!
//! let path = find_path(
//!     Pos::new(0, 0),
//!     Pos::new(9, 9),
//!     &grid,
//!     Heuristic::Manhattan,
//!     false, // 4-way movement
//! );
//!
//! if let Some(p) = path {
//!     println!("Found path with {} waypoints", p.len());
//! }
//! ```

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

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
        if self.is_valid(pos) {
            self.cost[pos.y as usize][pos.x as usize] = cost;
        }
    }

    /// Set a rectangular region as walkable/unwalkable
    pub fn set_region_walkable(&mut self, min: Pos, max: Pos, walkable: bool) {
        let min_x = min.x.max(0) as usize;
        let min_y = min.y.max(0) as usize;
        let max_x = (max.x as usize).min(self.width);
        let max_y = (max.y as usize).min(self.height);

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
        let max_x = (max.x as usize).min(self.width);
        let max_y = (max.y as usize).min(self.height);

        for y in min_y..max_y {
            for x in min_x..max_x {
                self.cost[y][x] = cost;
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

/// Node in the A* priority queue
#[derive(Debug, Clone)]
struct AStarNode {
    pos: Pos,
    f_score: f32,
    g_score: f32,
}

impl Eq for AStarNode {}

impl PartialEq for AStarNode {
    fn eq(&self, other: &Self) -> bool {
        self.f_score == other.f_score
    }
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f_score
            .partial_cmp(&self.f_score)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Heuristic function type for A*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heuristic {
    /// Manhattan distance (best for 4-way movement)
    Manhattan,
    /// Euclidean distance (best for 8-way movement)
    Euclidean,
}

impl Heuristic {
    fn estimate(&self, from: Pos, to: Pos) -> f32 {
        match self {
            Heuristic::Manhattan => from.manhattan_distance(&to) as f32,
            Heuristic::Euclidean => from.euclidean_distance(&to),
        }
    }
}

/// Find a path from start to goal using A* algorithm
///
/// # Parameters
/// - `start`: Starting position
/// - `goal`: Target position
/// - `grid`: The pathfinding grid with walkability info
/// - `heuristic`: Heuristic function to use
/// - `allow_diagonals`: Whether to allow diagonal movement
///
/// # Returns
/// `Some(Path)` if a path was found, `None` otherwise
pub fn find_path(
    start: Pos,
    goal: Pos,
    grid: &PathfindingGrid,
    heuristic: Heuristic,
    allow_diagonals: bool,
) -> Option<Path> {
    // Early exit checks
    if !grid.is_walkable(start) || !grid.is_walkable(goal) {
        return None;
    }

    if start == goal {
        return Some(Path::new(vec![start], 0.0));
    }

    // A* data structures
    let mut open_set = BinaryHeap::new();
    let mut came_from: HashMap<Pos, Pos> = HashMap::new();
    let mut g_scores: HashMap<Pos, f32> = HashMap::new();
    let mut closed_set: HashSet<Pos> = HashSet::new();

    // Initialize start node
    g_scores.insert(start, 0.0);
    open_set.push(AStarNode {
        pos: start,
        f_score: heuristic.estimate(start, goal),
        g_score: 0.0,
    });

    while let Some(current_node) = open_set.pop() {
        let current = current_node.pos;

        // Goal reached
        if current == goal {
            return Some(reconstruct_path(&came_from, current, current_node.g_score));
        }

        // Skip if already processed
        if closed_set.contains(&current) {
            continue;
        }

        closed_set.insert(current);

        // Get neighbors based on movement mode
        let neighbors: Vec<Pos> = if allow_diagonals {
            current.neighbors_8way().to_vec()
        } else {
            current.neighbors_4way().to_vec()
        };

        for neighbor in neighbors {
            // Skip invalid or unwalkable positions
            if !grid.is_walkable(neighbor) || closed_set.contains(&neighbor) {
                continue;
            }

            // Calculate movement cost
            let move_cost = if allow_diagonals {
                let is_diagonal =
                    (current.x - neighbor.x).abs() + (current.y - neighbor.y).abs() == 2;
                if is_diagonal {
                    1.414 * grid.get_cost(neighbor)
                } else {
                    grid.get_cost(neighbor)
                }
            } else {
                grid.get_cost(neighbor)
            };

            let tentative_g_score = current_node.g_score + move_cost;

            let neighbor_g_score = g_scores.get(&neighbor).copied().unwrap_or(f32::INFINITY);

            if tentative_g_score < neighbor_g_score {
                came_from.insert(neighbor, current);
                g_scores.insert(neighbor, tentative_g_score);

                let f_score = tentative_g_score + heuristic.estimate(neighbor, goal);

                open_set.push(AStarNode {
                    pos: neighbor,
                    f_score,
                    g_score: tentative_g_score,
                });
            }
        }
    }

    None
}

/// Find a path using closure-based walkability and movement costs.
///
/// This avoids materializing a `PathfindingGrid` when a game already has its
/// own grid or tile storage.
#[allow(clippy::too_many_arguments)]
pub fn find_path_with<FWalkable, FCost>(
    start: Pos,
    goal: Pos,
    width: usize,
    height: usize,
    is_walkable: FWalkable,
    movement_cost: FCost,
    heuristic: Heuristic,
    allow_diagonals: bool,
) -> Option<Path>
where
    FWalkable: Fn(Pos) -> bool,
    FCost: Fn(Pos) -> f32,
{
    let is_valid = |pos: Pos| {
        pos.x >= 0 && pos.y >= 0 && (pos.x as usize) < width && (pos.y as usize) < height
    };

    if !is_valid(start) || !is_valid(goal) || !is_walkable(start) || !is_walkable(goal) {
        return None;
    }

    if start == goal {
        return Some(Path::new(vec![start], 0.0));
    }

    let mut open_set = BinaryHeap::new();
    let mut came_from: HashMap<Pos, Pos> = HashMap::new();
    let mut g_scores: HashMap<Pos, f32> = HashMap::new();
    let mut closed_set: HashSet<Pos> = HashSet::new();

    g_scores.insert(start, 0.0);
    open_set.push(AStarNode {
        pos: start,
        f_score: heuristic.estimate(start, goal),
        g_score: 0.0,
    });

    while let Some(current_node) = open_set.pop() {
        let current = current_node.pos;

        if current == goal {
            return Some(reconstruct_path(&came_from, current, current_node.g_score));
        }

        if closed_set.contains(&current) {
            continue;
        }

        closed_set.insert(current);

        let neighbors: Vec<Pos> = if allow_diagonals {
            current.neighbors_8way().to_vec()
        } else {
            current.neighbors_4way().to_vec()
        };

        for neighbor in neighbors {
            if !is_valid(neighbor) || !is_walkable(neighbor) || closed_set.contains(&neighbor) {
                continue;
            }

            let base_cost = movement_cost(neighbor);
            if !base_cost.is_finite() {
                continue;
            }

            let is_diagonal = (current.x - neighbor.x).abs() + (current.y - neighbor.y).abs() == 2;
            let move_cost = if allow_diagonals && is_diagonal {
                1.414 * base_cost
            } else {
                base_cost
            };

            let tentative_g_score = current_node.g_score + move_cost;
            let neighbor_g_score = g_scores.get(&neighbor).copied().unwrap_or(f32::INFINITY);

            if tentative_g_score < neighbor_g_score {
                came_from.insert(neighbor, current);
                g_scores.insert(neighbor, tentative_g_score);

                open_set.push(AStarNode {
                    pos: neighbor,
                    f_score: tentative_g_score + heuristic.estimate(neighbor, goal),
                    g_score: tentative_g_score,
                });
            }
        }
    }

    None
}

/// Reconstruct path from came_from map
fn reconstruct_path(came_from: &HashMap<Pos, Pos>, mut current: Pos, total_cost: f32) -> Path {
    let mut path = vec![current];

    while let Some(&parent) = came_from.get(&current) {
        path.push(parent);
        current = parent;
    }

    path.reverse();
    Path::new(path, total_cost)
}

/// Path cache for avoiding repeated pathfinding calculations
pub struct PathCache {
    cache: HashMap<(Pos, Pos), Path>,
    invalidated_positions: HashSet<Pos>,
    max_cache_size: usize,
}

impl PathCache {
    /// Create a new path cache
    pub fn new(max_cache_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            invalidated_positions: HashSet::new(),
            max_cache_size,
        }
    }

    /// Find a path with caching
    pub fn find_path_cached(
        &mut self,
        start: Pos,
        goal: Pos,
        grid: &PathfindingGrid,
        heuristic: Heuristic,
        allow_diagonals: bool,
    ) -> Option<Path> {
        let key = (start, goal);

        if let Some(cached_path) = self.cache.get(&key) {
            let is_invalid = cached_path
                .waypoints
                .iter()
                .any(|pos| self.invalidated_positions.contains(pos));

            if !is_invalid {
                return Some(cached_path.clone());
            } else {
                self.cache.remove(&key);
            }
        }

        if let Some(path) = find_path(start, goal, grid, heuristic, allow_diagonals) {
            if self.cache.len() < self.max_cache_size {
                self.cache.insert(key, path.clone());
            }
            Some(path)
        } else {
            None
        }
    }

    /// Invalidate paths that pass through specific positions
    pub fn invalidate_positions(&mut self, positions: &[Pos]) {
        for pos in positions {
            self.invalidated_positions.insert(*pos);
        }

        self.cache.retain(|_, path| {
            !path
                .waypoints
                .iter()
                .any(|pos| self.invalidated_positions.contains(pos))
        });
    }

    /// Invalidate a single position
    pub fn invalidate_position(&mut self, pos: Pos) {
        self.invalidate_positions(&[pos]);
    }

    /// Clear all invalidated positions tracking
    pub fn clear_invalidations(&mut self) {
        self.invalidated_positions.clear();
    }

    /// Clear entire cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.invalidated_positions.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            cached_paths: self.cache.len(),
            invalidated_positions: self.invalidated_positions.len(),
            max_size: self.max_cache_size,
        }
    }
}

impl Default for PathCache {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub cached_paths: usize,
    pub invalidated_positions: usize,
    pub max_size: usize,
}

#[cfg(test)]
mod tests;
