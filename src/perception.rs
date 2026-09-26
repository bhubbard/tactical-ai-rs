use crate::grid::{CellType, Grid2D};
use crate::influence::{InfluenceLayer, InfluenceMap2D};
use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Vision cone model for tactical agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisionCone {
    pub origin: Vec2,
    pub direction: Vec2,
    /// Total field of view arc in radians (e.g. PI/2 = 90 degrees).
    pub fov_angle_rad: f32,
    pub max_distance: f32,
}

impl VisionCone {
    pub fn new(origin: Vec2, direction: Vec2, fov_angle_rad: f32, max_distance: f32) -> Self {
        let dir = if direction.length_squared() > 1e-6 {
            direction.normalize()
        } else {
            Vec2::X
        };
        Self {
            origin,
            direction: dir,
            fov_angle_rad,
            max_distance,
        }
    }

    /// Tests if a world point lies within the geometric vision cone (ignoring obstacles).
    pub fn contains_point(&self, point: Vec2) -> bool {
        let to_point = point - self.origin;
        let dist = to_point.length();
        if dist > self.max_distance || dist < 1e-4 {
            return dist <= self.max_distance;
        }

        let dir_to_point = to_point / dist;
        let cos_angle = self.direction.dot(dir_to_point).clamp(-1.0, 1.0);
        let angle = cos_angle.acos();
        angle <= (self.fov_angle_rad * 0.5)
    }

    /// Evaluates if an agent can see a target position, accounting for both cone arc and blocking obstacles.
    pub fn can_see_target(&self, target: Vec2, grid: &Grid2D<CellType>) -> bool {
        if !self.contains_point(target) {
            return false;
        }

        let start_coord = match grid.world_to_grid(self.origin) {
            Some(c) => c,
            None => return false,
        };
        let target_coord = match grid.world_to_grid(target) {
            Some(c) => c,
            None => return false,
        };

        grid.has_line_of_sight(start_coord, target_coord, |cell| cell.is_blocking())
    }

    /// Projects this vision cone onto an influence map's visibility layer.
    pub fn rasterize_to_influence(
        &self,
        grid: &Grid2D<CellType>,
        influence: &mut InfluenceMap2D,
        intensity: f32,
    ) {
        let cell_radius = (self.max_distance / grid.cell_size).ceil() as i32;
        let origin_coord = match grid.world_to_grid(self.origin) {
            Some(c) => c,
            None => return,
        };

        let min_x = (origin_coord.x - cell_radius).max(0);
        let max_x = (origin_coord.x + cell_radius).min(grid.width as i32 - 1);
        let min_y = (origin_coord.y - cell_radius).max(0);
        let max_y = (origin_coord.y + cell_radius).min(grid.height as i32 - 1);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let coord = crate::grid::GridCoord2D::new(x, y);
                let cell_pos = grid.grid_to_world_center(coord);
                if self.can_see_target(cell_pos, grid) {
                    let dist = self.origin.distance(cell_pos);
                    let falloff = (1.0 - dist / self.max_distance).max(0.0);
                    influence.add_influence(coord, InfluenceLayer::Visibility, intensity * falloff);
                }
            }
        }
    }
}

/// An acoustic impulse in the tactical environment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundEvent {
    pub origin: Vec2,
    /// Acoustic intensity at origin (arbitrary units or decibels).
    pub initial_intensity: f32,
    /// Maximum distance sound can travel before becoming zero.
    pub max_radius: f32,
}

impl SoundEvent {
    pub fn new(origin: Vec2, initial_intensity: f32, max_radius: f32) -> Self {
        Self {
            origin,
            initial_intensity,
            max_radius,
        }
    }
}

/// Auditory perception evaluator with acoustic attenuation through geometry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HearingModel {
    /// Percentage attenuation applied per blocking obstacle cell encountered (0.0 to 1.0).
    pub obstacle_attenuation: f32,
    /// Minimum threshold intensity required to perceive sound.
    pub perception_threshold: f32,
}

impl Default for HearingModel {
    fn default() -> Self {
        Self {
            obstacle_attenuation: 0.6, // 60% intensity loss per solid obstacle
            perception_threshold: 0.05,
        }
    }
}

impl HearingModel {
    pub fn new(obstacle_attenuation: f32, perception_threshold: f32) -> Self {
        Self {
            obstacle_attenuation: obstacle_attenuation.clamp(0.0, 1.0),
            perception_threshold,
        }
    }

    /// Calculates perceived sound intensity at `listener_pos`.
    /// Returns `Some(perceived_intensity)` if audible, or `None` if below threshold or out of range.
    pub fn evaluate_audibility(
        &self,
        sound: &SoundEvent,
        listener_pos: Vec2,
        grid: &Grid2D<CellType>,
    ) -> Option<f32> {
        let dist = sound.origin.distance(listener_pos);
        if dist > sound.max_radius || sound.max_radius <= 0.0 {
            return None;
        }

        // Distance geometric falloff (linear to zero at max_radius)
        let distance_factor = (1.0 - dist / sound.max_radius).max(0.0);
        let mut intensity = sound.initial_intensity * distance_factor;

        let start_coord = grid.world_to_grid(sound.origin);
        let end_coord = grid.world_to_grid(listener_pos);

        if let (Some(start), Some(end)) = (start_coord, end_coord) {
            let line = grid.bresenham_line(start, end);
            let mut obstacle_count = 0;
            for coord in line.into_iter().skip(1) {
                if coord == end {
                    break;
                }
                if let Some(cell) = grid.get_coord(coord) {
                    if cell.is_blocking() {
                        obstacle_count += 1;
                    }
                }
            }

            // Attenuate for each obstacle passed through
            for _ in 0..obstacle_count {
                intensity *= 1.0 - self.obstacle_attenuation;
            }
        }

        if intensity >= self.perception_threshold {
            Some(intensity)
        } else {
            None
        }
    }
}
