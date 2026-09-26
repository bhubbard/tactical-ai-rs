//! # Tactical AI RS
//!
//! Pure Rust tactical influence maps, spatial reasoning, cover finding, and flanking pathfinder.
//! Based on Game AI Pro Influence Maps & Tactical Spatial Reasoning.
//!
//! ## Core Systems
//! - **Spatial Grids (`grid`)**: 2D and 3D discrete regular grids with world-to-grid coordinate transforms and neighborhood queries.
//! - **Influence Maps (`influence`)**: Threat, Friendly, Tension, and Objective layers with linear, exponential, and Gaussian falloff, plus diffusion and decay simulation.
//! - **Cover & Vantage Finder (`cover`)**: Evaluates cover quality against threats, vantage line-of-sight to target areas, and fast nearest safe cover queries.
//! - **Flanking Pathfinding (`tactical_path`)**: Influence-biased A* pathfinder generating wide flanking routes and stealth infiltrations.
//! - **Sensory Perception (`perception`)**: Vision cones, Bresenham raycasting line-of-sight, and sound stimulus propagation with obstacle attenuation.

pub mod cover;
pub mod grid;
pub mod influence;
pub mod perception;
pub mod tactical_path;

pub use cover::{CoverFinder, CoverPoint, CoverQuality, CoverWeights};
pub use grid::{CellType, Grid2D, Grid3D, GridCoord2D, GridCoord3D};
pub use influence::{FalloffType, InfluenceLayer, InfluenceMap2D, InfluenceSource};
pub use perception::{HearingModel, SoundEvent, VisionCone};
pub use tactical_path::{FlankingPathfinder, PathResult, PathWeights};

use thiserror::Error;

/// Error types for tactical AI spatial reasoning operations.
#[derive(Debug, Error, PartialEq)]
pub enum TacticalError {
    #[error("Grid coordinates ({x}, {y}) out of bounds for grid size ({width}, {height})")]
    OutOfBounds2D {
        x: i32,
        y: i32,
        width: usize,
        height: usize,
    },

    #[error("Grid dimensions must be positive non-zero, got ({width}, {height})")]
    InvalidDimensions { width: usize, height: usize },

    #[error("Pathfinding failed: no valid path found from start to goal")]
    PathNotFound,

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}
