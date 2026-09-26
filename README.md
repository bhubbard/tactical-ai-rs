# tactical-ai-rs

[![GitHub Pages](https://img.shields.io/badge/Demo-Live%20Sandbox-06b6d4?style=flat-square&logo=github)](https://bhubbard.github.io/tactical-ai-rs/)
[![Rust](https://img.shields.io/badge/Rust-Edition%202024-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-blue?style=flat-square)](LICENSE)

Pure Rust implementation of **Tactical Influence Maps**, **Spatial Reasoning**, **Cover & Vantage Point Analysis**, and **Influence-Biased Flanking Pathfinding**, based on *Game AI Pro: Collected Wisdom of Game AI Professionals*.

👉 **[Try the Interactive 2D Tactical Grid Sandbox](https://bhubbard.github.io/tactical-ai-rs/)**

---

## ⚡ Features

- **Discrete Spatial Grids (`grid`)**:
  - High-performance 2D and 3D discrete regular grids.
  - Seamless world $\leftrightarrow$ grid coordinate transformations (`glam::Vec2` / `glam::Vec3`).
  - Bresenham line raycasting and unobstructed line-of-sight checks.
  - 4-way, 8-way (with diagonal $\sqrt{2}$ cost), and 3D 6-way/26-way neighborhood traversals.
- **Multi-Layer Influence Maps (`influence`)**:
  - Dedicated tactical semantic layers: `Threat`, `Friendly`, `Visibility`, `Value/Objective`, `Tension` ($|I_{\text{friendly}} - I_{\text{enemy}}|$), and `Vulnerability` ($\max(0, I_{\text{enemy}} - I_{\text{friendly}})$).
  - Configurable propagation falloff models:
    - **Linear**: $I(d) = I_0 \cdot \max(0, 1 - d/R)$
    - **Exponential**: $I(d) = I_0 \cdot \exp(-\lambda d)$
    - **Gaussian**: $I(d) = I_0 \cdot \exp(-(d/\sigma)^2)$
  - Physical diffusion and decay simulation:
    $$I_{t+1} = (1 - \gamma) I_t + \kappa \nabla^2 I_t$$
    where $\kappa \nabla^2 I_t$ is the discrete 4-neighborhood Laplacian operator, obeying geometric wall boundaries.
  - Gradient field computation ($\nabla I$) for continuous steering forces and avoidance fields.
- **Tactical Cover & Vantage Finder (`cover`)**:
  - Scans environment for cells offering obstacle protection against incoming threat vectors.
  - Evaluates cover score (blocked threat LOS), vantage score (unobstructed sightlines onto target positions), and agent proximity.
  - Fast single-pass queries: `find_nearest_safe_cover`, `find_best_cover`.
- **Flanking Route & Tactical Pathfinding (`tactical_path`)**:
  - Influence-biased A* pathfinder weighting danger vs distance:
    $$\text{cost}(u \to v) = \text{dist}(u, v) + w_{\text{threat}} \cdot I_{\text{threat}}(v) + w_{\text{vis}} \cdot I_{\text{vis}}(v) - w_{\text{cover}} \cdot I_{\text{cover}}(v)$$
  - Generates wide, realistic flanking maneuvers around enemy lines-of-sight instead of rushing straight into danger.
  - Raycast string-pulling shortcut smoothing for natural movement waypoints.
- **Sensory Perception (`perception`)**:
  - Field-of-View (FOV) vision cones (origin, orientation, aperture angle, max distance).
  - Raycast occlusion checks against physical obstacles.
  - Auditory sound stimulus propagation with acoustic decibel attenuation through solid geometry.

---

## 📦 Installation

Add `tactical-ai-rs` to your `Cargo.toml`:

```toml
[dependencies]
tactical-ai-rs = "0.1.0"
glam = "0.29"
```

---

## 🚀 Quick Start

### 1. Generating Tactical Influence & Tension

```rust
use glam::Vec2;
use tactical_ai_rs::{
    FalloffType, InfluenceLayer, InfluenceMap2D, InfluenceSource,
};

// Create a 40x40 tactical map with 1.0 unit cell size
let mut map = InfluenceMap2D::new(40, 40, 1.0, Vec2::ZERO);

// Stamp friendly base presence
map.stamp_source(
    &InfluenceSource::new(
        Vec2::new(10.0, 20.0),
        10.0,
        FalloffType::Linear { max_radius: 12.0 },
        InfluenceLayer::Friendly,
    ),
    None,
);

// Stamp enemy threat presence
map.stamp_source(
    &InfluenceSource::new(
        Vec2::new(30.0, 20.0),
        15.0,
        FalloffType::Exponential { lambda: 0.15, max_radius: 14.0 },
        InfluenceLayer::Threat,
    ),
    None,
);

// Advance simulation tick: 5% decay, 15% Laplacian diffusion
map.diffuse_and_decay(InfluenceLayer::Threat, 0.05, 0.15, None);
```

### 2. Finding Safe Cover Under Fire

```rust
use glam::Vec2;
use tactical_ai_rs::{CellType, CoverFinder, CoverWeights, Grid2D};

let mut grid = Grid2D::new(30, 30, 1.0, Vec2::ZERO, CellType::Empty);
// Construct obstacle wall
for y in 10..20 {
    *grid.get_mut(15, y).unwrap() = CellType::FullCover;
}

let finder = CoverFinder::new(&grid);
let agent_pos = Vec2::new(10.0, 15.0);
let threat_pos = Vec2::new(22.0, 15.0);
let enemy_target = Vec2::new(22.0, 12.0);

if let Some(best_cover) = finder.find_best_cover(
    agent_pos,
    &[threat_pos],
    &[enemy_target],
    &CoverWeights::default(),
    15.0,
) {
    println!("Best tactical cover at: {:?}", best_cover.world_pos);
    println!("Cover Score (LOS blocked): {:.2}", best_cover.cover_score);
    println!("Vantage Score (sight to target): {:.2}", best_cover.vantage_score);
}
```

### 3. Calculating an Influence-Biased Flanking Route

```rust
use glam::Vec2;
use tactical_ai_rs::{CellType, FlankingPathfinder, Grid2D, InfluenceMap2D, PathWeights};

let grid = Grid2D::new(50, 50, 1.0, Vec2::ZERO, CellType::Empty);
let influence = InfluenceMap2D::new(50, 50, 1.0, Vec2::ZERO);

let pathfinder = FlankingPathfinder::new(&grid).with_influence(&influence);

// Stealth flanking weights heavily penalize traveling through threat influence
let flank_route = pathfinder.find_path(
    Vec2::new(5.0, 25.0),
    Vec2::new(45.0, 25.0),
    &PathWeights::stealth_flanking(),
).expect("Route found");

println!("Waypoints: {:?}", flank_route.waypoints);
println!("Average Threat Encountered: {:.2}", flank_route.average_threat);
```

---

## 🧪 Running Tests

```bash
cargo test
```

All 8 integration tests verify:
- Grid transformations & 3D spatial queries
- Falloff curves (Linear, Exponential, Gaussian)
- Discrete Laplacian diffusion and decay
- Multi-layer tension and vulnerability
- Cover & vantage point scoring
- Flanking route avoidance around threat bubbles
- Vision cone raycasting and acoustic obstacle attenuation

---

## 📄 License

Licensed under either of [Apache License, Version 2.0](LICENSE) or [MIT License](LICENSE) at your option.
