use super::*;

#[test]
fn test_manhattan_distance() {
    let a = Pos::new(0, 0);
    let b = Pos::new(3, 4);
    assert_eq!(a.manhattan_distance(&b), 7);
}

#[test]
fn test_straight_line_path() {
    let grid = PathfindingGrid::new(10, 10);
    let start = Pos::new(0, 0);
    let goal = Pos::new(5, 0);

    let path = find_path(start, goal, &grid, Heuristic::Manhattan, false);
    assert!(path.is_some());

    let path = path.unwrap();
    assert_eq!(path.len(), 6);
    assert_eq!(path.start(), Some(start));
    assert_eq!(path.goal(), Some(goal));
}

#[test]
fn test_path_around_obstacle() {
    let mut grid = PathfindingGrid::new(5, 5);

    for y in 0..4 {
        grid.set_walkable(Pos::new(2, y), false);
    }

    let start = Pos::new(0, 2);
    let goal = Pos::new(4, 2);

    let path = find_path(start, goal, &grid, Heuristic::Manhattan, false);
    assert!(path.is_some());

    let path = path.unwrap();
    assert!(!path.waypoints.contains(&Pos::new(2, 0)));
    assert!(!path.waypoints.contains(&Pos::new(2, 1)));
    assert!(!path.waypoints.contains(&Pos::new(2, 2)));
}

#[test]
fn test_no_path() {
    let mut grid = PathfindingGrid::new(5, 5);

    for y in 0..5 {
        grid.set_walkable(Pos::new(2, y), false);
    }

    let start = Pos::new(0, 2);
    let goal = Pos::new(4, 2);

    let path = find_path(start, goal, &grid, Heuristic::Manhattan, false);
    assert!(path.is_none());
}

#[test]
fn test_diagonal_movement() {
    let grid = PathfindingGrid::new(10, 10);
    let start = Pos::new(0, 0);
    let goal = Pos::new(5, 5);

    let path_diagonal = find_path(start, goal, &grid, Heuristic::Euclidean, true);
    let path_4way = find_path(start, goal, &grid, Heuristic::Manhattan, false);

    assert!(path_diagonal.is_some());
    assert!(path_4way.is_some());

    assert!(path_diagonal.unwrap().len() < path_4way.unwrap().len());
}

#[test]
fn test_closure_based_pathfinding() {
    let blocked = [
        Pos::new(2, 0),
        Pos::new(2, 1),
        Pos::new(2, 2),
        Pos::new(2, 3),
    ];
    let path = find_path_with(
        Pos::new(0, 2),
        Pos::new(4, 2),
        5,
        5,
        |pos| !blocked.contains(&pos),
        |_| 1.0,
        Heuristic::Manhattan,
        false,
    )
    .unwrap();

    assert_eq!(path.start(), Some(Pos::new(0, 2)));
    assert_eq!(path.goal(), Some(Pos::new(4, 2)));
    assert!(path.waypoints.contains(&Pos::new(2, 4)));
}

#[test]
fn cache_keeps_search_modes_separate() {
    let grid = PathfindingGrid::new(10, 10);
    let mut cache = PathCache::new(8);
    let start = Pos::new(0, 0);
    let goal = Pos::new(5, 5);

    let diagonal = cache
        .find_path_cached(start, goal, &grid, Heuristic::Euclidean, true)
        .expect("the diagonal route exists");
    let four_way = cache
        .find_path_cached(start, goal, &grid, Heuristic::Manhattan, false)
        .expect("the four-way route exists");

    assert!(diagonal.len() < four_way.len());
    assert_eq!(cache.stats().cached_paths, 2);
}

#[test]
fn invalid_grid_costs_are_ignored_instead_of_poisoning_the_grid() {
    let mut grid = PathfindingGrid::new(3, 1);
    let middle = Pos::new(1, 0);

    grid.set_cost(middle, f32::NAN);
    assert_eq!(grid.get_cost(middle), 1.0);
    grid.set_cost(middle, -1.0);
    assert_eq!(grid.get_cost(middle), 1.0);
}

#[test]
fn negative_region_bounds_do_not_wrap_to_the_far_edge() {
    let mut grid = PathfindingGrid::new(3, 3);

    grid.set_region_walkable(Pos::new(0, 0), Pos::new(-1, 2), false);
    grid.set_region_cost(Pos::new(0, 0), Pos::new(-1, 2), 4.0);

    assert!(grid.is_walkable(Pos::new(0, 0)));
    assert_eq!(grid.get_cost(Pos::new(0, 0)), 1.0);
    assert!(grid.is_walkable(Pos::new(2, 2)));
}
