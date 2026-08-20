use super::{find_path, Heuristic, Path, PathfindingGrid, Pos};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
struct CacheKey {
    start: Pos,
    goal: Pos,
    heuristic: Heuristic,
    allow_diagonals: bool,
}

/// Path cache for avoiding repeated pathfinding calculations
pub struct PathCache {
    cache: HashMap<CacheKey, Path>,
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
        let key = CacheKey {
            start,
            goal,
            heuristic,
            allow_diagonals,
        };

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
