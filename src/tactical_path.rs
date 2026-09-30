use crate::grid::{CellType, Grid2D, GridCoord2D};
use crate::influence::{InfluenceLayer, InfluenceMap2D};
use crate::TacticalError;
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// Weights controlling how the A* pathfinder balances distance vs tactical hazards.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PathWeights {
    /// Cost multiplier for physical travel distance.
    pub weight_distance: f32,
    /// Cost multiplier for passing through high threat influence.
    pub weight_threat: f32,
    /// Cost reduction for moving along cover walls.
    pub weight_cover: f32,
    /// Cost multiplier for moving in enemy visual sight.
    pub weight_visibility: f32,
}

impl Default for PathWeights {
    fn default() -> Self {
        Self {
            weight_distance: 1.0,
            weight_threat: 6.0,
            weight_cover: 1.5,
            weight_visibility: 3.0,
        }
    }
}

impl PathWeights {
    /// Direct shortest path ignoring tactical threat.
    pub fn shortest_distance() -> Self {
        Self {
            weight_distance: 1.0,
            weight_threat: 0.0,
            weight_cover: 0.0,
            weight_visibility: 0.0,
        }
    }

    /// Aggressively wide flanking maneuver prioritizing staying undetected.
    pub fn stealth_flanking() -> Self {
        Self {
            weight_distance: 0.8,
            weight_threat: 15.0,
            weight_cover: 3.0,
            weight_visibility: 8.0,
        }
    }
}

/// Computed tactical route result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathResult {
    /// Smoothed world-space waypoints for agent locomotion.
    pub waypoints: Vec<Vec2>,
    /// Raw discrete grid cell coordinates traversed.
    pub grid_path: Vec<GridCoord2D>,
    /// Accumulated tactical traversal cost.
    pub total_cost: f32,
    /// Euclidean path distance in world units.
    pub total_distance: f32,
    /// Average threat level experienced along the path.
    pub average_threat: f32,
}

#[derive(Copy, Clone, PartialEq)]
struct AStarNode {
    coord: GridCoord2D,
    f_score: f32,
}

impl Eq for AStarNode {}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap
        other.f_score.partial_cmp(&self.f_score).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Tactical A* and Dijkstra pathfinder utilizing influence maps.
#[derive(Debug)]
pub struct FlankingPathfinder<'a> {
    pub grid: &'a Grid2D<CellType>,
    pub influence: Option<&'a InfluenceMap2D>,
}

impl<'a> FlankingPathfinder<'a> {
    pub fn new(grid: &'a Grid2D<CellType>) -> Self {
        Self {
            grid,
            influence: None,
        }
    }

    pub fn with_influence(mut self, influence: &'a InfluenceMap2D) -> Self {
        self.influence = Some(influence);
        self
    }

    /// Calculates a tactical path from start to goal in world coordinates.
    pub fn find_path(
        &self,
        start_world: Vec2,
        goal_world: Vec2,
        weights: &PathWeights,
    ) -> Result<PathResult, TacticalError> {
        let start_coord = self
            .grid
            .world_to_grid(start_world)
            .ok_or(TacticalError::OutOfBounds2D {
                x: start_world.x as i32,
                y: start_world.y as i32,
                width: self.grid.width,
                height: self.grid.height,
            })?;

        let goal_coord = self
            .grid
            .world_to_grid(goal_world)
            .ok_or(TacticalError::OutOfBounds2D {
                x: goal_world.x as i32,
                y: goal_world.y as i32,
                width: self.grid.width,
                height: self.grid.height,
            })?;

        self.find_grid_path(start_coord, goal_coord, weights)
    }

    /// Runs influence-biased A* on the discrete grid.
    pub fn find_grid_path(
        &self,
        start: GridCoord2D,
        goal: GridCoord2D,
        weights: &PathWeights,
    ) -> Result<PathResult, TacticalError> {
        if !self.grid.in_bounds_coord(start) || !self.grid.in_bounds_coord(goal) {
            return Err(TacticalError::OutOfBounds2D {
                x: start.x,
                y: start.y,
                width: self.grid.width,
                height: self.grid.height,
            });
        }

        // Verify goal is walkable
        if let Some(cell) = self.grid.get_coord(goal) {
            if !cell.is_walkable() {
                return Err(TacticalError::PathNotFound);
            }
        }

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<GridCoord2D, GridCoord2D> = HashMap::new();
        let mut g_score: HashMap<GridCoord2D, f32> = HashMap::new();

        g_score.insert(start, 0.0);
        let h_start = start.distance(goal) * self.grid.cell_size * weights.weight_distance;
        open_set.push(AStarNode {
            coord: start,
            f_score: h_start,
        });

        while let Some(AStarNode { coord: current, .. }) = open_set.pop() {
            if current == goal {
                // Reconstruct path
                let mut path = vec![current];
                let mut curr = current;
                while let Some(&prev) = came_from.get(&curr) {
                    path.push(prev);
                    curr = prev;
                }
                path.reverse();

                let total_cost = *g_score.get(&goal).unwrap_or(&0.0);
                return Ok(self.build_result(path, total_cost));
            }

            let current_g = match g_score.get(&current) {
                Some(&g) => g,
                None => continue,
            };

            for (neighbor, step_dist_cells) in self.grid.neighbors_8(current) {
                if let Some(cell) = self.grid.get_coord(neighbor) {
                    if !cell.is_walkable() {
                        continue;
                    }
                }

                // Prevent cutting through diagonal walls
                if (neighbor.x != current.x) && (neighbor.y != current.y) {
                    let c1 = GridCoord2D::new(neighbor.x, current.y);
                    let c2 = GridCoord2D::new(current.x, neighbor.y);
                    let b1 = self.grid.get_coord(c1).map_or(true, |c| !c.is_walkable());
                    let b2 = self.grid.get_coord(c2).map_or(true, |c| !c.is_walkable());
                    if b1 && b2 {
                        continue;
                    }
                }

                let step_dist_world = step_dist_cells * self.grid.cell_size;
                let step_cost = self.evaluate_step_cost(neighbor, step_dist_world, weights);
                let tentative_g = current_g + step_cost;

                let prev_g = g_score.get(&neighbor).copied().unwrap_or(f32::INFINITY);
                if tentative_g < prev_g {
                    came_from.insert(neighbor, current);
                    g_score.insert(neighbor, tentative_g);
                    let h = neighbor.distance(goal) * self.grid.cell_size * weights.weight_distance;
                    open_set.push(AStarNode {
                        coord: neighbor,
                        f_score: tentative_g + h,
                    });
                }
            }
        }

        Err(TacticalError::PathNotFound)
    }

    fn evaluate_step_cost(&self, coord: GridCoord2D, dist: f32, weights: &PathWeights) -> f32 {
        let base_cost = dist * weights.weight_distance;

        let (threat, visibility, is_near_cover) = if let Some(inf) = self.influence {
            let t = inf.get_influence(coord, InfluenceLayer::Threat);
            let v = inf.get_influence(coord, InfluenceLayer::Visibility);
            (t, v, false)
        } else {
            (0.0, 0.0, false)
        };

        let cover_bonus = if is_near_cover { weights.weight_cover } else { 0.0 };

        (base_cost + weights.weight_threat * threat + weights.weight_visibility * visibility - cover_bonus)
            .max(0.01)
    }

    fn build_result(&self, grid_path: Vec<GridCoord2D>, total_cost: f32) -> PathResult {
        let mut total_distance = 0.0;
        let mut threat_sum = 0.0;

        let world_points: Vec<Vec2> = grid_path
            .iter()
            .map(|&c| self.grid.grid_to_world_center(c))
            .collect();

        for i in 1..world_points.len() {
            total_distance += world_points[i - 1].distance(world_points[i]);
        }

        if let Some(inf) = self.influence {
            for &c in &grid_path {
                threat_sum += inf.get_influence(c, InfluenceLayer::Threat);
            }
        }

        let average_threat = if grid_path.is_empty() {
            0.0
        } else {
            threat_sum / grid_path.len() as f32
        };

        // Smooth world waypoints using raycast string pulling
        let waypoints = self.smooth_path(&world_points);

        PathResult {
            waypoints,
            grid_path,
            total_cost,
            total_distance,
            average_threat,
        }
    }

    /// Shortcut path smoothing: drops redundant intermediate collinear/LOS waypoints.
    pub fn smooth_path(&self, points: &[Vec2]) -> Vec<Vec2> {
        if points.len() <= 2 {
            return points.to_vec();
        }

        let mut smoothed = Vec::new();
        smoothed.push(points[0]);
        let mut anchor_idx = 0;

        while anchor_idx < points.len() - 1 {
            let mut furthest = anchor_idx + 1;
            for test_idx in (anchor_idx + 2..points.len()).rev() {
                if self.has_world_los(points[anchor_idx], points[test_idx]) {
                    furthest = test_idx;
                    break;
                }
            }
            smoothed.push(points[furthest]);
            anchor_idx = furthest;
        }

        smoothed
    }

    fn has_world_los(&self, a: Vec2, b: Vec2) -> bool {
        let ca = match self.grid.world_to_grid(a) {
            Some(c) => c,
            None => return false,
        };
        let cb = match self.grid.world_to_grid(b) {
            Some(c) => c,
            None => return false,
        };
        self.grid.has_line_of_sight(ca, cb, |c| !c.is_walkable())
    }
}
