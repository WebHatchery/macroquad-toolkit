//! A* pathfinding for grid-based games.
//!
//! The public API is kept in one small facade while the data model, search
//! algorithm, and cache each live in their own module.

mod cache;
mod search;
mod types;

pub use cache::*;
pub use search::{find_path, find_path_with};
pub use types::*;

#[cfg(test)]
mod tests;
