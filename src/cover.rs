use crate::grid::{CellType, Grid2D, GridCoord2D};
use crate::influence::{InfluenceLayer, InfluenceMap2D};
use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Qualitative classification of tactical cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CoverQuality {
    None,
    Low,
    Medium,
    High,
}

/// A tactically evaluated cover location.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverPoint {
    pub coord: GridCoord2D,
    pub world_pos: Vec2,
    /// Score for blocking threat line of sight [0.0, 1.0].
    pub cover_score: f32,
    /// Score for vantage over targets [0.0, 1.0].
    pub vantage_score: f32,
    /// Proximity distance from querying agent.
    pub distance_to_agent: f32,
    /// Threat influence level at this position.
    pub threat_exposure: f32,
    /// Weighted composite tactical score (higher is better).
    pub composite_score: f32,
}

/// Scoring weights for tactical cover evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CoverWeights {
    pub weight_cover: f32,
    pub weight_vantage: f32,
    pub weight_proximity: f32,
    pub weight_threat_exposure: f32,
}

impl Default for CoverWeights {
    fn default() -> Self {
        Self {
            weight_cover: 2.0,
            weight_vantage: 1.0,
            weight_proximity: 0.5,
            weight_threat_exposure: 1.5,
        }
    }
}

/// Evaluator for identifying and ranking tactical cover and vantage positions.
#[derive(Debug)]
pub struct CoverFinder<'a> {
    pub grid: &'a Grid2D<CellType>,
    pub influence_map: Option<&'a InfluenceMap2D>,
}

impl<'a> CoverFinder<'a> {
    pub fn new(grid: &'a Grid2D<CellType>) -> Self {
        Self {
            grid,
            influence_map: None,
        }
    }

    pub fn with_influence(mut self, influence: &'a InfluenceMap2D) -> Self {
        self.influence_map = Some(influence);
        self
    }

    /// Evaluates if a given walkable cell has adjacent obstacle protection.
    pub fn is_cover_candidate(&self, coord: GridCoord2D) -> bool {
        if let Some(cell) = self.grid.get_coord(coord) {
            if !cell.is_walkable() {
                return false;
            }
            // Candidate must be adjacent to at least one cover/obstacle cell
            for n in self.grid.neighbors_4(coord) {
                if let Some(n_cell) = self.grid.get_coord(n) {
                    if n_cell.provides_cover() || n_cell.is_blocking() {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Finds and scores all cover positions relative to given threats and optional target area.
    pub fn find_all_cover(
        &self,
        agent_pos: Vec2,
        threat_positions: &[Vec2],
        target_positions: &[Vec2],
        weights: &CoverWeights,
        max_search_radius: f32,
    ) -> Vec<CoverPoint> {
        let agent_coord = match self.grid.world_to_grid(agent_pos) {
            Some(c) => c,
            None => return Vec::new(),
        };

        let radius_cells = (max_search_radius / self.grid.cell_size).ceil() as i32;
        let min_x = (agent_coord.x - radius_cells).max(0);
        let max_x = (agent_coord.x + radius_cells).min(self.grid.width as i32 - 1);
        let min_y = (agent_coord.y - radius_cells).max(0);
        let max_y = (agent_coord.y + radius_cells).min(self.grid.height as i32 - 1);

        let mut points = Vec::new();

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let coord = GridCoord2D::new(x, y);
                if !self.is_cover_candidate(coord) {
                    continue;
                }

                let world_pos = self.grid.grid_to_world_center(coord);
                let dist_to_agent = agent_pos.distance(world_pos);
                if dist_to_agent > max_search_radius {
                    continue;
                }

                // 1. Cover score: fraction of threats whose line of sight is BLOCKED to this cell
                let cover_score = if threat_positions.is_empty() {
                    1.0
                } else {
                    let mut blocked_count = 0;
                    for &threat in threat_positions {
                        if let Some(threat_coord) = self.grid.world_to_grid(threat) {
                            let has_los = self.grid.has_line_of_sight(
                                threat_coord,
                                coord,
                                |c| c.is_blocking() || c.provides_cover(),
                            );
                            if !has_los {
                                blocked_count += 1;
                            }
                        }
                    }
                    blocked_count as f32 / threat_positions.len() as f32
                };

                // 2. Vantage score: fraction of targets that this cell HAS line of sight towards
                let vantage_score = if target_positions.is_empty() {
                    0.5
                } else {
                    let mut visible_count = 0;
                    for &target in target_positions {
                        if let Some(target_coord) = self.grid.world_to_grid(target) {
                            let has_los = self.grid.has_line_of_sight(
                                coord,
                                target_coord,
                                |c| c.is_blocking(),
                            );
                            if has_los {
                                visible_count += 1;
                            }
                        }
                    }
                    visible_count as f32 / target_positions.len() as f32
                };

                // 3. Threat exposure from influence map
                let threat_exposure = self
                    .influence_map
                    .map(|inf| inf.get_influence(coord, InfluenceLayer::Threat))
                    .unwrap_or(0.0);

                // 4. Normalized proximity factor: closer to agent is better
                let proximity_factor = 1.0 - (dist_to_agent / max_search_radius).clamp(0.0, 1.0);

                // Composite score calculation
                let composite_score = weights.weight_cover * cover_score
                    + weights.weight_vantage * vantage_score
                    + weights.weight_proximity * proximity_factor
                    - weights.weight_threat_exposure * threat_exposure;

                points.push(CoverPoint {
                    coord,
                    world_pos,
                    cover_score,
                    vantage_score,
                    distance_to_agent: dist_to_agent,
                    threat_exposure,
                    composite_score,
                });
            }
        }

        // Sort descending by composite score
        points.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap_or(std::cmp::Ordering::Equal));
        points
    }

    /// Finds the highest-ranked cover point according to weights.
    pub fn find_best_cover(
        &self,
        agent_pos: Vec2,
        threat_positions: &[Vec2],
        target_positions: &[Vec2],
        weights: &CoverWeights,
        max_search_radius: f32,
    ) -> Option<CoverPoint> {
        self.find_all_cover(agent_pos, threat_positions, target_positions, weights, max_search_radius)
            .into_iter()
            .next()
    }

    /// Finds the nearest cover that completely breaks LOS from all threats.
    pub fn find_nearest_safe_cover(
        &self,
        agent_pos: Vec2,
        threat_positions: &[Vec2],
        max_search_radius: f32,
    ) -> Option<CoverPoint> {
        let weights = CoverWeights {
            weight_cover: 5.0,
            weight_vantage: 0.0,
            weight_proximity: 2.0,
            weight_threat_exposure: 1.0,
        };
        let covers = self.find_all_cover(agent_pos, threat_positions, &[], &weights, max_search_radius);
        covers.into_iter().filter(|c| c.cover_score >= 0.99).min_by(|a, b| {
            a.distance_to_agent.partial_cmp(&b.distance_to_agent).unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}
