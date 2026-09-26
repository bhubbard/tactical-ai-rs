use std::time::Instant;
use glam::Vec2;
use tactical_ai_rs::{
    CellType, CoverFinder, CoverWeights, FalloffType, FlankingPathfinder, Grid2D,
    GridCoord2D, InfluenceLayer, InfluenceMap2D, InfluenceSource, PathWeights,
};

fn main() {
    println!("============================================================");
    println!("     tactical-ai-rs (Rust) vs Game AI Pro Unity (C#) Bench  ");
    println!("============================================================");

    // 1. Influence Map Source Stamping & Layer Aggregation
    println!("\n--- 1. Influence Map Multi-Source Stamping & Diffusion (50x50 Grid) ---");
    {
        let mut map = InfluenceMap2D::new(50, 50, 1.0, Vec2::ZERO);

        let sources: Vec<InfluenceSource> = (0..16)
            .map(|i| {
                let pos = Vec2::new((i % 4) as f32 * 12.0 + 5.0, (i / 4) as f32 * 12.0 + 5.0);
                let layer = if i % 2 == 0 { InfluenceLayer::Threat } else { InfluenceLayer::Friendly };
                InfluenceSource::new(pos, 25.0, FalloffType::Gaussian { sigma: 6.0, max_radius: 12.0 }, layer)
            })
            .collect();

        let iterations = 10_000;
        let start = Instant::now();

        for _ in 0..iterations {
            map.clear();
            for src in &sources {
                map.stamp_source(src, None);
            }
            map.diffuse_and_decay(InfluenceLayer::Threat, 0.15, 0.05, None);
            map.diffuse_and_decay(InfluenceLayer::Friendly, 0.15, 0.05, None);
        }

        let elapsed = start.elapsed();
        let us_per_tick = elapsed.as_micros() as f64 / iterations as f64;
        let ticks_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "Influence Map Ticks (16 Sources + Diffusion): {} | Time: {:.2?} | Latency: {:.2} µs/tick | {:>10.0} ticks/s",
            iterations, elapsed, us_per_tick, ticks_per_sec
        );
    }

    // 2. Cover Quality & Threat Occlusion Search
    println!("\n--- 2. Cover Finder Occlusion & Vantage Scoring (100x100 Grid) ---");
    {
        let mut grid = Grid2D::new(100, 100, 1.0, Vec2::ZERO, CellType::Empty);
        // Place multiple cover pillars and walls
        for x in (10..90).step_by(10) {
            for y in 20..80 {
                *grid.get_mut(x, y).unwrap() = CellType::FullCover;
            }
        }

        let finder = CoverFinder::new(&grid);
        let weights = CoverWeights::default();

        let threats = [
            Vec2::new(55.0, 50.0),
            Vec2::new(75.0, 30.0),
        ];
        let vantages = [
            Vec2::new(50.0, 50.0),
        ];

        let iterations = 20_000;
        let start = Instant::now();
        let mut found_count = 0;

        for i in 0..iterations {
            let agent_pos = Vec2::new(15.0 + (i % 30) as f32, 25.0 + (i % 50) as f32);
            if let Some(_cover) = finder.find_best_cover(agent_pos, &threats, &vantages, &weights, 25.0) {
                found_count += 1;
            }
        }

        let elapsed = start.elapsed();
        let us_per_search = elapsed.as_micros() as f64 / iterations as f64;
        let searches_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "Cover Searches: {} | Time: {:.2?} | Latency: {:.2} µs/search | {:>10.0} searches/s | Found: {}",
            iterations, elapsed, us_per_search, searches_per_sec, found_count
        );
    }

    // 3. Tactical Influence-Biased Flanking A* Pathfinding
    println!("\n--- 3. Flanking Pathfinder (A* with Threat Avoidance Weights) ---");
    {
        let mut grid = Grid2D::new(60, 60, 1.0, Vec2::ZERO, CellType::Empty);
        // Central obstacle
        for y in 20..40 {
            *grid.get_mut(30, y).unwrap() = CellType::FullCover;
        }

        let mut map = InfluenceMap2D::new(60, 60, 1.0, Vec2::ZERO);
        // Enemy threat field covering center
        let threat = InfluenceSource::new(
            Vec2::new(30.0, 30.0),
            50.0,
            FalloffType::Linear { max_radius: 20.0 },
            InfluenceLayer::Threat,
        );
        map.stamp_source(&threat, None);

        let pathfinder = FlankingPathfinder::new(&grid).with_influence(&map);
        let weights = PathWeights::stealth_flanking();

        let start_pos = Vec2::new(10.5, 30.5);
        let goal_pos = Vec2::new(50.5, 30.5);

        let iterations = 50_000;
        let start = Instant::now();
        let mut total_path_len = 0;

        for _ in 0..iterations {
            if let Ok(res) = pathfinder.find_path(start_pos, goal_pos, &weights) {
                total_path_len += res.waypoints.len();
            }
        }

        let elapsed = start.elapsed();
        let us_per_path = elapsed.as_micros() as f64 / iterations as f64;
        let paths_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "Flanking Paths: {} | Time: {:.2?} | Latency: {:.2} µs/path | {:>10.0} paths/s | Avg Nodes: {}",
            iterations, elapsed, us_per_path, paths_per_sec, total_path_len / iterations
        );
    }

    // 4. Bresenham Line-of-Sight & Vision Cone Raycasting
    println!("\n--- 4. Bresenham Line-of-Sight Grid Checks ---");
    {
        let grid = Grid2D::new(100, 100, 1.0, Vec2::ZERO, CellType::Empty);
        let iterations = 1_000_000;
        let start = Instant::now();
        let mut total_cells = 0;

        for i in 0..iterations {
            let start_c = GridCoord2D::new((i % 40) as i32, 10);
            let end_c = GridCoord2D::new(50 + (i % 40) as i32, 90);
            let line = grid.bresenham_line(start_c, end_c);
            total_cells += line.len();
        }

        let elapsed = start.elapsed();
        let ns_per_ray = elapsed.as_nanos() as f64 / iterations as f64;
        let rays_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "Bresenham Rays: {} | Time: {:.2?} | Latency: {:.2} ns/ray ({:>10.0} rays/s) | Total Cells: {}",
            iterations, elapsed, ns_per_ray, rays_per_sec, total_cells
        );
    }

    println!("\n============================================================");
    println!("                      Benchmark Complete                    ");
    println!("============================================================");
}
