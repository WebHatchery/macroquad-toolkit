//! Grid utilities for tile-based games
//!
//! Provides generic grid data structures and coordinate conversion utilities
//! for both 2D orthogonal and isometric grid systems.
//!
//! # Example
//! ```
//! use macroquad_toolkit::grid::{Grid, TilePos, world_to_iso, iso_to_world};
//!
//! // Create a grid
//! let mut grid: Grid<i32> = Grid::new(10, 10, 0);
//! grid.set(TilePos::new(5, 5), 1);
//!
//! // Coordinate conversion
//! let (iso_x, iso_y) = world_to_iso(5.0, 3.0, 64.0, 32.0);
//! ```

mod flat_grid;
mod fog;
mod iso;
mod pathfinding;
mod tile_pos;
mod vec_grid;
mod vision;

pub use flat_grid::FlatGrid;
pub use fog::{update_flat_fog_states, update_fog_states, FogState};
pub use iso::{iso_to_world, world_to_iso};
pub use pathfinding::{bfs_path, flood_fill, reachable_within};
pub use tile_pos::TilePos;
pub use vec_grid::Grid;
pub use vision::{calculate_visible_tiles, has_line_of_sight, line_positions, tiles_in_radius};

#[cfg(test)]
mod tests;
