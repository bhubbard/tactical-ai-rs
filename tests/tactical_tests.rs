use glam::{Vec2, Vec3};
use tactical_ai_rs::{
    CellType, CoverFinder, CoverWeights, FalloffType, FlankingPathfinder, Grid2D, Grid3D,
    GridCoord2D, GridCoord3D, HearingModel, InfluenceLayer, InfluenceMap2D, InfluenceSource,
    PathWeights, SoundEvent, VisionCone,
};

#[test]
fn test_grid_coordinates_and_conversions() {
    let grid = Grid2D::new(10, 10, 2.0, Vec2::new(0.0, 0.0), CellType::Empty);
    assert_eq!(grid.width, 10);
    assert_eq!(grid.height, 10);
    assert_eq!(grid.cell_size, 2.0);

    let coord = grid.world_to_grid(Vec2::new(3.5, 5.0)).unwrap();
    assert_eq!(coord, GridCoord2D::new(1, 2));

    let center = grid.grid_to_world_center(GridCoord2D::new(1, 2));
    assert!((center.x - 3.0).abs() < 1e-4);
    assert!((center.y - 5.0).abs() < 1e-4);

    let line = grid.bresenham_line(GridCoord2D::new(0, 0), GridCoord2D::new(3, 0));
    assert_eq!(line.len(), 4);
    assert_eq!(line[0], GridCoord2D::new(0, 0));
    assert_eq!(line[3], GridCoord2D::new(3, 0));
}

#[test]
fn test_grid_3d_basics() {
    let grid3d = Grid3D::new(5, 5, 5, 1.0, Vec3::ZERO, 0i32);
    assert!(grid3d.in_bounds(2, 2, 2));
    assert!(!grid3d.in_bounds(5, 2, 2));

    let neighbors = grid3d.neighbors_6(GridCoord3D::new(2, 2, 2));
    assert_eq!(neighbors.len(), 6);
}

#[test]
fn test_influence_falloff_models() {
    let linear = FalloffType::Linear { max_radius: 10.0 };
    assert!((linear.evaluate(0.0, 100.0) - 100.0).abs() < 1e-4);
    assert!((linear.evaluate(5.0, 100.0) - 50.0).abs() < 1e-4);
    assert_eq!(linear.evaluate(10.0, 100.0), 0.0);
    assert_eq!(linear.evaluate(12.0, 100.0), 0.0);

    let exp = FalloffType::Exponential {
        lambda: 0.1,
        max_radius: 20.0,
    };
    assert!((exp.evaluate(0.0, 100.0) - 100.0).abs() < 1e-4);
    assert!(exp.evaluate(5.0, 100.0) < 100.0);
    assert!(exp.evaluate(5.0, 100.0) > 0.0);

    let gaussian = FalloffType::Gaussian {
        sigma: 5.0,
        max_radius: 15.0,
    };
    assert!((gaussian.evaluate(0.0, 50.0) - 50.0).abs() < 1e-4);
    assert!(gaussian.evaluate(5.0, 50.0) < 50.0);
}

#[test]
fn test_influence_stamping_and_layers() {
    let mut map = InfluenceMap2D::new(20, 20, 1.0, Vec2::ZERO);

    // Friendly source at (5, 5)
    let friendly_src = InfluenceSource::new(
        Vec2::new(5.5, 5.5),
        10.0,
        FalloffType::Linear { max_radius: 5.0 },
        InfluenceLayer::Friendly,
    );
    map.stamp_source(&friendly_src, None);

    // Enemy threat source at (10, 5)
    let threat_src = InfluenceSource::new(
        Vec2::new(10.5, 5.5),
        10.0,
        FalloffType::Linear { max_radius: 5.0 },
        InfluenceLayer::Threat,
    );
    map.stamp_source(&threat_src, None);

    let f_val = map.get_influence(GridCoord2D::new(5, 5), InfluenceLayer::Friendly);
    assert!(f_val > 8.0);

    let t_val = map.get_influence(GridCoord2D::new(10, 5), InfluenceLayer::Threat);
    assert!(t_val > 8.0);

    // Midpoint tension
    let mid_coord = GridCoord2D::new(8, 5);
    let tension = map.get_influence(mid_coord, InfluenceLayer::Tension);
    let mid_f = map.get_influence(mid_coord, InfluenceLayer::Friendly);
    let mid_t = map.get_influence(mid_coord, InfluenceLayer::Threat);
    assert_eq!(tension, (mid_t - mid_f).abs());
}

#[test]
fn test_influence_diffusion_and_decay() {
    let mut map = InfluenceMap2D::new(10, 10, 1.0, Vec2::ZERO);
    let center = GridCoord2D::new(5, 5);
    map.set_influence(center, InfluenceLayer::Threat, 10.0);

    assert_eq!(map.get_influence(GridCoord2D::new(4, 5), InfluenceLayer::Threat), 0.0);

    // Perform one diffusion tick
    map.diffuse_and_decay(InfluenceLayer::Threat, 0.1, 0.2, None);

    let center_after = map.get_influence(center, InfluenceLayer::Threat);
    let neighbor_after = map.get_influence(GridCoord2D::new(4, 5), InfluenceLayer::Threat);

    assert!(center_after < 10.0);
    assert!(neighbor_after > 0.0);
}

#[test]
fn test_cover_finder_evaluation() {
    // 10x10 grid with a wall in the middle
    let mut grid = Grid2D::new(10, 10, 1.0, Vec2::ZERO, CellType::Empty);
    // Wall at x=5, y in [2..8]
    for y in 2..8 {
        *grid.get_mut(5, y).unwrap() = CellType::FullCover;
    }

    let finder = CoverFinder::new(&grid);
    let agent_pos = Vec2::new(3.5, 5.5);
    let threat_pos = Vec2::new(8.5, 5.5);
    let target_pos = Vec2::new(8.5, 1.5);

    let best_cover = finder
        .find_best_cover(
            agent_pos,
            &[threat_pos],
            &[target_pos],
            &CoverWeights::default(),
            10.0,
        )
        .expect("Should find cover near the wall");

    // The safe side of the wall from threat at (8.5, 5.5) is on x=4
    assert!(best_cover.cover_score > 0.9);
    assert_eq!(best_cover.coord.x, 4);

    let nearest_safe = finder.find_nearest_safe_cover(agent_pos, &[threat_pos], 10.0);
    assert!(nearest_safe.is_some());
    assert_eq!(nearest_safe.unwrap().coord.x, 4);
}

#[test]
fn test_flanking_pathfinding() {
    let grid = Grid2D::new(20, 20, 1.0, Vec2::ZERO, CellType::Empty);
    let mut influence = InfluenceMap2D::new(20, 20, 1.0, Vec2::ZERO);

    // Place an intense enemy threat zone in the center (10, 10)
    let threat_src = InfluenceSource::new(
        Vec2::new(10.5, 10.5),
        20.0,
        FalloffType::Linear { max_radius: 5.0 },
        InfluenceLayer::Threat,
    );
    influence.stamp_source(&threat_src, None);

    let pathfinder = FlankingPathfinder::new(&grid).with_influence(&influence);

    let start = Vec2::new(3.5, 10.5);
    let goal = Vec2::new(17.5, 10.5);

    // 1. Direct path with 0 threat weight should pass straight through center
    let direct_weights = PathWeights::shortest_distance();
    let direct_res = pathfinder.find_path(start, goal, &direct_weights).unwrap();
    let passes_through_center = direct_res.grid_path.iter().any(|c| (c.x - 10).abs() <= 1 && (c.y - 10).abs() <= 1);
    assert!(passes_through_center);

    // 2. Flanking path with high threat weight should detour around center
    let flanking_weights = PathWeights::stealth_flanking();
    let flanking_res = pathfinder.find_path(start, goal, &flanking_weights).unwrap();
    let detoured_center = flanking_res.grid_path.iter().all(|c| {
        let dist_to_threat = ((c.x - 10).pow(2) + (c.y - 10).pow(2)) as f32;
        dist_to_threat >= 9.0 // stays away from center threat core
    });
    assert!(detoured_center, "Flanking path must skirt around high threat");
    assert!(flanking_res.average_threat < direct_res.average_threat);
}

#[test]
fn test_sensory_perception() {
    let mut grid = Grid2D::new(20, 20, 1.0, Vec2::ZERO, CellType::Empty);
    // Add obstacle wall at x = 10, y in 0..10
    for y in 0..10 {
        *grid.get_mut(10, y).unwrap() = CellType::Obstacle;
    }

    let vision = VisionCone::new(
        Vec2::new(5.0, 5.0),
        Vec2::new(1.0, 0.0), // Facing East
        std::f32::consts::FRAC_PI_2, // 90 degree FOV
        15.0,
    );

    // In front within FOV and before wall
    assert!(vision.can_see_target(Vec2::new(8.0, 5.0), &grid));

    // Behind agent (outside FOV)
    assert!(!vision.can_see_target(Vec2::new(2.0, 5.0), &grid));

    // Behind wall (occluded by obstacle)
    assert!(!vision.can_see_target(Vec2::new(12.0, 5.0), &grid));

    // Hearing model test
    let hearing = HearingModel::default();
    let sound_clear = SoundEvent::new(Vec2::new(5.0, 15.0), 10.0, 20.0);
    // Listener at (15.0, 15.0) - no wall in between at y=15
    let heard_clear = hearing.evaluate_audibility(&sound_clear, Vec2::new(15.0, 15.0), &grid);
    assert!(heard_clear.is_some());

    // Listener at (15.0, 5.0) with sound at (5.0, 5.0) - passes through wall at x=10
    let sound_blocked = SoundEvent::new(Vec2::new(5.0, 5.0), 1.0, 20.0);
    let heard_blocked = hearing.evaluate_audibility(&sound_blocked, Vec2::new(15.0, 5.0), &grid);
    // Attenuation reduces 1.0 intensity through obstacle
    if let (Some(c), Some(b)) = (
        hearing.evaluate_audibility(&SoundEvent::new(Vec2::new(5.0, 15.0), 1.0, 20.0), Vec2::new(15.0, 15.0), &grid),
        heard_blocked,
    ) {
        assert!(b < c, "Sound through obstacle should have lower intensity than clear line");
    }
}
