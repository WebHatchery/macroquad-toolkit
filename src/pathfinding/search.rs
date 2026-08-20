//! Shared A* search implementation for grid and closure-backed maps.

use super::{Heuristic, Path, PathfindingGrid, Pos};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

#[derive(Debug, Clone)]
struct AStarNode {
    pos: Pos,
    f_score: f32,
    g_score: f32,
}

impl Eq for AStarNode {}

impl PartialEq for AStarNode {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos
            && self.f_score.to_bits() == other.f_score.to_bits()
            && self.g_score.to_bits() == other.g_score.to_bits()
    }
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f_score
            .total_cmp(&self.f_score)
            .then_with(|| other.g_score.total_cmp(&self.g_score))
            .then_with(|| other.pos.y.cmp(&self.pos.y))
            .then_with(|| other.pos.x.cmp(&self.pos.x))
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct SearchSpace<FWalkable, FCost> {
    width: usize,
    height: usize,
    is_walkable: FWalkable,
    movement_cost: FCost,
}

/// Find a path from start to goal using a pathfinding grid.
pub fn find_path(
    start: Pos,
    goal: Pos,
    grid: &PathfindingGrid,
    heuristic: Heuristic,
    allow_diagonals: bool,
) -> Option<Path> {
    let space = SearchSpace {
        width: grid.width,
        height: grid.height,
        is_walkable: |pos| grid.is_walkable(pos),
        movement_cost: |pos| grid.get_cost(pos),
    };
    find_path_impl(start, goal, space, heuristic, allow_diagonals)
}

/// Find a path using closure-based walkability and movement costs.
///
/// This avoids materializing a pathfinding grid when a game already has its
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
    let space = SearchSpace {
        width,
        height,
        is_walkable,
        movement_cost,
    };
    find_path_impl(start, goal, space, heuristic, allow_diagonals)
}

fn find_path_impl<FWalkable, FCost>(
    start: Pos,
    goal: Pos,
    space: SearchSpace<FWalkable, FCost>,
    heuristic: Heuristic,
    allow_diagonals: bool,
) -> Option<Path>
where
    FWalkable: Fn(Pos) -> bool,
    FCost: Fn(Pos) -> f32,
{
    let is_valid = |pos: Pos| {
        pos.x >= 0
            && pos.y >= 0
            && (pos.x as usize) < space.width
            && (pos.y as usize) < space.height
    };

    if !is_valid(start)
        || !is_valid(goal)
        || !(space.is_walkable)(start)
        || !(space.is_walkable)(goal)
    {
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

        let neighbor_count = if allow_diagonals { 8 } else { 4 };
        for index in 0..neighbor_count {
            let neighbor = if allow_diagonals {
                current.neighbors_8way()[index]
            } else {
                current.neighbors_4way()[index]
            };
            if !is_valid(neighbor)
                || !(space.is_walkable)(neighbor)
                || closed_set.contains(&neighbor)
            {
                continue;
            }

            let base_cost = (space.movement_cost)(neighbor);
            if !base_cost.is_finite() || base_cost < 0.0 {
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

fn reconstruct_path(came_from: &HashMap<Pos, Pos>, mut current: Pos, total_cost: f32) -> Path {
    let mut path = vec![current];
    while let Some(&parent) = came_from.get(&current) {
        path.push(parent);
        current = parent;
    }
    path.reverse();
    Path::new(path, total_cost)
}
