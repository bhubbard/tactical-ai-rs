use crate::grid::{CellType, Grid2D, GridCoord2D};
use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Falloff models for spatial influence propagation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FalloffType {
    /// Linear falloff: I(d) = I_0 * max(0, 1 - d / R)
    Linear { max_radius: f32 },
    /// Exponential falloff: I(d) = I_0 * exp(-lambda * d)
    Exponential { lambda: f32, max_radius: f32 },
    /// Gaussian falloff: I(d) = I_0 * exp(-(d / sigma)^2)
    Gaussian { sigma: f32, max_radius: f32 },
}

impl FalloffType {
    pub fn evaluate(&self, distance: f32, initial_intensity: f32) -> f32 {
        if distance < 0.0 {
            return initial_intensity;
        }
        match *self {
            FalloffType::Linear { max_radius } => {
                if distance >= max_radius || max_radius <= 0.0 {
                    0.0
                } else {
                    initial_intensity * (1.0 - distance / max_radius).max(0.0)
                }
            }
            FalloffType::Exponential { lambda, max_radius } => {
                if distance >= max_radius {
                    0.0
                } else {
                    initial_intensity * (-lambda * distance).exp()
                }
            }
            FalloffType::Gaussian { sigma, max_radius } => {
                if distance >= max_radius || sigma <= 0.0 {
                    0.0
                } else {
                    let normalized = distance / sigma;
                    initial_intensity * (-normalized * normalized).exp()
                }
            }
        }
    }

    pub fn max_radius(&self) -> f32 {
        match *self {
            FalloffType::Linear { max_radius } => max_radius,
            FalloffType::Exponential { max_radius, .. } => max_radius,
            FalloffType::Gaussian { max_radius, .. } => max_radius,
        }
    }
}

/// Identifies the tactical semantic layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InfluenceLayer {
    Threat,
    Friendly,
    Visibility,
    Value,
    Tension,
    Vulnerability,
}

/// A localized emitter of tactical influence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InfluenceSource {
    pub position: Vec2,
    pub intensity: f32,
    pub falloff: FalloffType,
    pub layer: InfluenceLayer,
    /// Whether propagation requires unoccluded line-of-sight.
    pub requires_los: bool,
}

impl InfluenceSource {
    pub fn new(position: Vec2, intensity: f32, falloff: FalloffType, layer: InfluenceLayer) -> Self {
        Self {
            position,
            intensity,
            falloff,
            layer,
            requires_los: false,
        }
    }

    pub fn with_los(mut self, requires_los: bool) -> Self {
        self.requires_los = requires_los;
        self
    }
}

/// Multi-layer 2D tactical influence map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InfluenceMap2D {
    pub width: usize,
    pub height: usize,
    pub cell_size: f32,
    pub origin: Vec2,
    threat: Grid2D<f32>,
    friendly: Grid2D<f32>,
    visibility: Grid2D<f32>,
    value: Grid2D<f32>,
}

impl InfluenceMap2D {
    pub fn new(width: usize, height: usize, cell_size: f32, origin: Vec2) -> Self {
        Self {
            width,
            height,
            cell_size,
            origin,
            threat: Grid2D::new(width, height, cell_size, origin, 0.0),
            friendly: Grid2D::new(width, height, cell_size, origin, 0.0),
            visibility: Grid2D::new(width, height, cell_size, origin, 0.0),
            value: Grid2D::new(width, height, cell_size, origin, 0.0),
        }
    }

    pub fn clear(&mut self) {
        for v in self.threat.as_mut_slice() {
            *v = 0.0;
        }
        for v in self.friendly.as_mut_slice() {
            *v = 0.0;
        }
        for v in self.visibility.as_mut_slice() {
            *v = 0.0;
        }
        for v in self.value.as_mut_slice() {
            *v = 0.0;
        }
    }

    pub fn clear_layer(&mut self, layer: InfluenceLayer) {
        let grid = match layer {
            InfluenceLayer::Threat => &mut self.threat,
            InfluenceLayer::Friendly => &mut self.friendly,
            InfluenceLayer::Visibility => &mut self.visibility,
            InfluenceLayer::Value => &mut self.value,
            InfluenceLayer::Tension | InfluenceLayer::Vulnerability => return,
        };
        for v in grid.as_mut_slice() {
            *v = 0.0;
        }
    }

    pub fn get_layer_grid(&self, layer: InfluenceLayer) -> &Grid2D<f32> {
        match layer {
            InfluenceLayer::Threat => &self.threat,
            InfluenceLayer::Friendly => &self.friendly,
            InfluenceLayer::Visibility => &self.visibility,
            InfluenceLayer::Value => &self.value,
            InfluenceLayer::Tension | InfluenceLayer::Vulnerability => &self.threat,
        }
    }

    pub fn get_layer_grid_mut(&mut self, layer: InfluenceLayer) -> Option<&mut Grid2D<f32>> {
        match layer {
            InfluenceLayer::Threat => Some(&mut self.threat),
            InfluenceLayer::Friendly => Some(&mut self.friendly),
            InfluenceLayer::Visibility => Some(&mut self.visibility),
            InfluenceLayer::Value => Some(&mut self.value),
            InfluenceLayer::Tension | InfluenceLayer::Vulnerability => None,
        }
    }

    /// Evaluates influence at a given grid cell for the specified layer.
    pub fn get_influence(&self, coord: GridCoord2D, layer: InfluenceLayer) -> f32 {
        if !self.threat.in_bounds_coord(coord) {
            return 0.0;
        }
        match layer {
            InfluenceLayer::Threat => *self.threat.get_coord(coord).unwrap_or(&0.0),
            InfluenceLayer::Friendly => *self.friendly.get_coord(coord).unwrap_or(&0.0),
            InfluenceLayer::Visibility => *self.visibility.get_coord(coord).unwrap_or(&0.0),
            InfluenceLayer::Value => *self.value.get_coord(coord).unwrap_or(&0.0),
            InfluenceLayer::Tension => {
                let t = *self.threat.get_coord(coord).unwrap_or(&0.0);
                let f = *self.friendly.get_coord(coord).unwrap_or(&0.0);
                (t - f).abs()
            }
            InfluenceLayer::Vulnerability => {
                let t = *self.threat.get_coord(coord).unwrap_or(&0.0);
                let f = *self.friendly.get_coord(coord).unwrap_or(&0.0);
                (t - f).max(0.0)
            }
        }
    }

    pub fn set_influence(&mut self, coord: GridCoord2D, layer: InfluenceLayer, value: f32) {
        if let Some(grid) = self.get_layer_grid_mut(layer) {
            if let Some(cell) = grid.get_coord_mut(coord) {
                *cell = value;
            }
        }
    }

    pub fn add_influence(&mut self, coord: GridCoord2D, layer: InfluenceLayer, delta: f32) {
        if let Some(grid) = self.get_layer_grid_mut(layer) {
            if let Some(cell) = grid.get_coord_mut(coord) {
                *cell += delta;
            }
        }
    }

    /// Adds influence from an emitter source onto the target layer.
    /// If obstacle_grid is provided and `source.requires_los` is true, cells behind obstacles are occluded.
    pub fn stamp_source(
        &mut self,
        source: &InfluenceSource,
        obstacle_grid: Option<&Grid2D<CellType>>,
    ) {
        let max_r = source.falloff.max_radius();
        let cell_radius = (max_r / self.cell_size).ceil() as i32;

        let center_coord = match self.threat.world_to_grid(source.position) {
            Some(c) => c,
            None => {
                let rel = source.position - self.origin;
                GridCoord2D::new(
                    (rel.x / self.cell_size).floor() as i32,
                    (rel.y / self.cell_size).floor() as i32,
                )
            }
        };

        let min_x = (center_coord.x - cell_radius).max(0);
        let max_x = (center_coord.x + cell_radius).min(self.width as i32 - 1);
        let min_y = (center_coord.y - cell_radius).max(0);
        let max_y = (center_coord.y + cell_radius).min(self.height as i32 - 1);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let coord = GridCoord2D::new(x, y);
                let world_cell = self.threat.grid_to_world_center(coord);
                let dist = source.position.distance(world_cell);
                if dist > max_r {
                    continue;
                }

                if source.requires_los {
                    if let Some(obstacles) = obstacle_grid {
                        if !obstacles.has_line_of_sight(center_coord, coord, |c| c.is_blocking()) {
                            continue;
                        }
                    }
                }

                let value = source.falloff.evaluate(dist, source.intensity);
                self.add_influence(coord, source.layer, value);
            }
        }
    }

    /// Simulates diffusion and decay per tick on the specified layer.
    ///
    /// Equation: I_{t+1} = (1 - gamma) * I_t + kappa * nabla^2 I_t
    ///
    /// - `decay_rate` (gamma): Fraction of influence dissipated per tick [0.0, 1.0].
    /// - `diffusion_rate` (kappa): Rate of spreading into adjacent cells [0.0, 0.25].
    /// - `obstacle_grid`: When specified, blocking obstacles do not conduct diffusion.
    pub fn diffuse_and_decay(
        &mut self,
        layer: InfluenceLayer,
        decay_rate: f32,
        diffusion_rate: f32,
        obstacle_grid: Option<&Grid2D<CellType>>,
    ) {
        let grid = match self.get_layer_grid_mut(layer) {
            Some(g) => g,
            None => return,
        };

        let w = grid.width;
        let h = grid.height;
        let mut next = vec![0.0f32; w * h];
        let current = grid.as_slice();

        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let c_val = current[idx];

                if let Some(obs) = obstacle_grid {
                    if let Some(cell) = obs.get(x as i32, y as i32) {
                        if cell.is_blocking() {
                            next[idx] = 0.0;
                            continue;
                        }
                    }
                }

                // Compute discrete Laplacian: nabla^2 I = sum(neighbors) - 4 * I
                let mut neighbor_sum = 0.0f32;
                let mut valid_neighbors = 0;

                let neighbors = [
                    (x as i32 + 1, y as i32),
                    (x as i32 - 1, y as i32),
                    (x as i32, y as i32 + 1),
                    (x as i32, y as i32 - 1),
                ];

                for (nx, ny) in neighbors {
                    if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                        let n_idx = (ny as usize) * w + (nx as usize);
                        let is_blocked = obstacle_grid.map_or(false, |obs| {
                            obs.get(nx, ny).map_or(false, |c| c.is_blocking())
                        });

                        if !is_blocked {
                            neighbor_sum += current[n_idx];
                            valid_neighbors += 1;
                        }
                    }
                }

                let laplacian = if valid_neighbors > 0 {
                    neighbor_sum - (valid_neighbors as f32) * c_val
                } else {
                    0.0
                };

                let decayed = (1.0 - decay_rate.clamp(0.0, 1.0)) * c_val;
                let diffused = diffusion_rate.clamp(0.0, 0.25) * laplacian;
                let val = (decayed + diffused).max(0.0);
                next[idx] = val;
            }
        }

        grid.as_mut_slice().copy_from_slice(&next);
    }

    /// Computes the spatial gradient (nabla I) at a grid coordinate for steering vector fields.
    pub fn gradient(&self, coord: GridCoord2D, layer: InfluenceLayer) -> Vec2 {
        let x = coord.x;
        let y = coord.y;
        let left = self.get_influence(GridCoord2D::new(x - 1, y), layer);
        let right = self.get_influence(GridCoord2D::new(x + 1, y), layer);
        let down = self.get_influence(GridCoord2D::new(x, y - 1), layer);
        let up = self.get_influence(GridCoord2D::new(x, y + 1), layer);

        let dx = (right - left) / (2.0 * self.cell_size);
        let dy = (up - down) / (2.0 * self.cell_size);
        Vec2::new(dx, dy)
    }

    /// Returns the maximum value present across a layer.
    pub fn max_value(&self, layer: InfluenceLayer) -> f32 {
        let grid = self.get_layer_grid(layer);
        grid.as_slice()
            .iter()
            .copied()
            .fold(0.0f32, |acc, v| acc.max(v))
    }

    /// Normalizes the given layer into [0.0, 1.0].
    pub fn normalize(&mut self, layer: InfluenceLayer) {
        let max = self.max_value(layer);
        if max > 0.0 {
            if let Some(grid) = self.get_layer_grid_mut(layer) {
                for v in grid.as_mut_slice() {
                    *v /= max;
                }
            }
        }
    }
}
