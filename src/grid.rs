use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// 2D discrete integer grid coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GridCoord2D {
    pub x: i32,
    pub y: i32,
}

impl GridCoord2D {
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn distance_squared(self, other: Self) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        dx * dx + dy * dy
    }

    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        self.distance_squared(other).sqrt()
    }

    #[inline]
    pub fn manhattan_distance(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    #[inline]
    pub fn chebyshev_distance(self, other: Self) -> i32 {
        (self.x - other.x).abs().max((self.y - other.y).abs())
    }
}

/// 3D discrete integer grid coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GridCoord3D {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl GridCoord3D {
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn distance_squared(self, other: Self) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        let dz = (self.z - other.z) as f32;
        dx * dx + dy * dy + dz * dz
    }

    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        self.distance_squared(other).sqrt()
    }

    #[inline]
    pub fn manhattan_distance(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()
    }
}

/// Tactical cell classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CellType {
    #[default]
    Empty,
    Obstacle,
    HalfCover,
    FullCover,
    Hazard,
}

impl CellType {
    #[inline]
    pub fn is_walkable(self) -> bool {
        matches!(self, CellType::Empty | CellType::Hazard)
    }

    #[inline]
    pub fn is_blocking(self) -> bool {
        matches!(self, CellType::Obstacle | CellType::FullCover)
    }

    #[inline]
    pub fn provides_cover(self) -> bool {
        matches!(self, CellType::HalfCover | CellType::FullCover)
    }
}

/// 2D regular discrete spatial grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grid2D<T> {
    pub width: usize,
    pub height: usize,
    pub cell_size: f32,
    pub origin: Vec2,
    cells: Vec<T>,
}

impl<T: Clone> Grid2D<T> {
    /// Creates a new 2D grid filled with `default_value`.
    pub fn new(width: usize, height: usize, cell_size: f32, origin: Vec2, default_value: T) -> Self {
        assert!(width > 0 && height > 0, "Grid dimensions must be > 0");
        assert!(cell_size > 0.0, "Cell size must be > 0.0");
        let total_cells = width * height;
        Self {
            width,
            height,
            cell_size,
            origin,
            cells: vec![default_value; total_cells],
        }
    }

    /// Creates a new 2D grid initialized using a generator closure.
    pub fn from_fn<F>(width: usize, height: usize, cell_size: f32, origin: Vec2, mut f: F) -> Self
    where
        F: FnMut(usize, usize) -> T,
    {
        assert!(width > 0 && height > 0, "Grid dimensions must be > 0");
        assert!(cell_size > 0.0, "Cell size must be > 0.0");
        let mut cells = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                cells.push(f(x, y));
            }
        }
        Self {
            width,
            height,
            cell_size,
            origin,
            cells,
        }
    }
}

impl<T> Grid2D<T> {
    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && (x as usize) < self.width && y >= 0 && (y as usize) < self.height
    }

    #[inline]
    pub fn in_bounds_coord(&self, coord: GridCoord2D) -> bool {
        self.in_bounds(coord.x, coord.y)
    }

    #[inline]
    pub fn index_of(&self, x: i32, y: i32) -> Option<usize> {
        if self.in_bounds(x, y) {
            Some((y as usize) * self.width + (x as usize))
        } else {
            None
        }
    }

    #[inline]
    pub fn coord_of(&self, index: usize) -> Option<GridCoord2D> {
        if index < self.cells.len() {
            let x = (index % self.width) as i32;
            let y = (index / self.width) as i32;
            Some(GridCoord2D::new(x, y))
        } else {
            None
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Option<&T> {
        self.index_of(x, y).map(|idx| &self.cells[idx])
    }

    #[inline]
    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut T> {
        if self.in_bounds(x, y) {
            let idx = (y as usize) * self.width + (x as usize);
            Some(&mut self.cells[idx])
        } else {
            None
        }
    }

    #[inline]
    pub fn get_coord(&self, coord: GridCoord2D) -> Option<&T> {
        self.get(coord.x, coord.y)
    }

    #[inline]
    pub fn get_coord_mut(&mut self, coord: GridCoord2D) -> Option<&mut T> {
        self.get_mut(coord.x, coord.y)
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.cells
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.cells
    }

    /// Converts world coordinates to grid cell coordinates.
    pub fn world_to_grid(&self, world_pos: Vec2) -> Option<GridCoord2D> {
        let rel = world_pos - self.origin;
        let gx = (rel.x / self.cell_size).floor() as i32;
        let gy = (rel.y / self.cell_size).floor() as i32;
        if self.in_bounds(gx, gy) {
            Some(GridCoord2D::new(gx, gy))
        } else {
            None
        }
    }

    /// Converts grid coordinates to world center coordinates of the cell.
    pub fn grid_to_world_center(&self, coord: GridCoord2D) -> Vec2 {
        self.origin
            + Vec2::new(
                (coord.x as f32 + 0.5) * self.cell_size,
                (coord.y as f32 + 0.5) * self.cell_size,
            )
    }

    /// Converts grid coordinates to world top-left/min coordinates of the cell.
    pub fn grid_to_world_min(&self, coord: GridCoord2D) -> Vec2 {
        self.origin + Vec2::new(coord.x as f32 * self.cell_size, coord.y as f32 * self.cell_size)
    }

    /// Returns valid 4-way orthogonal neighbors (N, S, E, W).
    pub fn neighbors_4(&self, coord: GridCoord2D) -> Vec<GridCoord2D> {
        let mut neighbors = Vec::with_capacity(4);
        let deltas = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        for (dx, dy) in deltas {
            let nx = coord.x + dx;
            let ny = coord.y + dy;
            if self.in_bounds(nx, ny) {
                neighbors.push(GridCoord2D::new(nx, ny));
            }
        }
        neighbors
    }

    /// Returns valid 8-way neighbors with movement cost (1.0 for orthogonal, sqrt(2) for diagonal).
    pub fn neighbors_8(&self, coord: GridCoord2D) -> Vec<(GridCoord2D, f32)> {
        let mut neighbors = Vec::with_capacity(8);
        let sqrt2 = std::f32::consts::SQRT_2;
        let deltas = [
            (0, 1, 1.0),
            (0, -1, 1.0),
            (1, 0, 1.0),
            (-1, 0, 1.0),
            (1, 1, sqrt2),
            (1, -1, sqrt2),
            (-1, 1, sqrt2),
            (-1, -1, sqrt2),
        ];
        for (dx, dy, cost) in deltas {
            let nx = coord.x + dx;
            let ny = coord.y + dy;
            if self.in_bounds(nx, ny) {
                neighbors.push((GridCoord2D::new(nx, ny), cost));
            }
        }
        neighbors
    }

    /// Computes all grid cells traversed along a line from start to end using Bresenham's algorithm.
    pub fn bresenham_line(&self, start: GridCoord2D, end: GridCoord2D) -> Vec<GridCoord2D> {
        let mut line = Vec::new();
        let mut x0 = start.x;
        let mut y0 = start.y;
        let x1 = end.x;
        let y1 = end.y;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if self.in_bounds(x0, y0) {
                line.push(GridCoord2D::new(x0, y0));
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }

        line
    }

    /// Checks if there is a line of sight between start and end cell.
    /// Excludes the start cell from obstacle checking.
    pub fn has_line_of_sight<F>(&self, start: GridCoord2D, end: GridCoord2D, is_obstacle: F) -> bool
    where
        F: Fn(&T) -> bool,
    {
        let line = self.bresenham_line(start, end);
        // Skip first cell (start point)
        for coord in line.into_iter().skip(1) {
            if let Some(cell) = self.get_coord(coord) {
                if coord == end {
                    // Reached target
                    return true;
                }
                if is_obstacle(cell) {
                    return false;
                }
            }
        }
        true
    }
}

/// 3D regular discrete spatial grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grid3D<T> {
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub cell_size: f32,
    pub origin: Vec3,
    cells: Vec<T>,
}

impl<T: Clone> Grid3D<T> {
    pub fn new(
        width: usize,
        height: usize,
        depth: usize,
        cell_size: f32,
        origin: Vec3,
        default_value: T,
    ) -> Self {
        assert!(width > 0 && height > 0 && depth > 0, "Dimensions must be > 0");
        assert!(cell_size > 0.0, "Cell size must be > 0.0");
        let total_cells = width * height * depth;
        Self {
            width,
            height,
            depth,
            cell_size,
            origin,
            cells: vec![default_value; total_cells],
        }
    }
}

impl<T> Grid3D<T> {
    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32, z: i32) -> bool {
        x >= 0
            && (x as usize) < self.width
            && y >= 0
            && (y as usize) < self.height
            && z >= 0
            && (z as usize) < self.depth
    }

    #[inline]
    pub fn index_of(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        if self.in_bounds(x, y, z) {
            Some(
                ((z as usize) * self.height + (y as usize)) * self.width
                    + (x as usize),
            )
        } else {
            None
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> Option<&T> {
        self.index_of(x, y, z).map(|idx| &self.cells[idx])
    }

    #[inline]
    pub fn get_mut(&mut self, x: i32, y: i32, z: i32) -> Option<&mut T> {
        if self.in_bounds(x, y, z) {
            let idx = ((z as usize) * self.height + (y as usize)) * self.width
                + (x as usize);
            Some(&mut self.cells[idx])
        } else {
            None
        }
    }

    pub fn world_to_grid(&self, world_pos: Vec3) -> Option<GridCoord3D> {
        let rel = world_pos - self.origin;
        let gx = (rel.x / self.cell_size).floor() as i32;
        let gy = (rel.y / self.cell_size).floor() as i32;
        let gz = (rel.z / self.cell_size).floor() as i32;
        if self.in_bounds(gx, gy, gz) {
            Some(GridCoord3D::new(gx, gy, gz))
        } else {
            None
        }
    }

    pub fn grid_to_world_center(&self, coord: GridCoord3D) -> Vec3 {
        self.origin
            + Vec3::new(
                (coord.x as f32 + 0.5) * self.cell_size,
                (coord.y as f32 + 0.5) * self.cell_size,
                (coord.z as f32 + 0.5) * self.cell_size,
            )
    }

    pub fn neighbors_6(&self, coord: GridCoord3D) -> Vec<GridCoord3D> {
        let mut neighbors = Vec::with_capacity(6);
        let deltas = [
            (1, 0, 0),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
            (0, 0, 1),
            (0, 0, -1),
        ];
        for (dx, dy, dz) in deltas {
            let nx = coord.x + dx;
            let ny = coord.y + dy;
            let nz = coord.z + dz;
            if self.in_bounds(nx, ny, nz) {
                neighbors.push(GridCoord3D::new(nx, ny, nz));
            }
        }
        neighbors
    }
}
