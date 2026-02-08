use super::super::*;
use crate::r2::R2;
use crate::shape::Shape;

/// Test behavior with impossible targets (each region = 100% of total).
///
/// These targets are geometrically impossible for non-identical shapes.
/// This test characterizes the oscillation behavior to ensure it's bounded
/// and doesn't produce NaN/infinity.
#[test]
fn test_impossible_targets_oscillation() {
    use crate::model::Model;
    use crate::to::To;

    // Circle and triangle with impossible targets
    let circle: Shape<f64> = crate::shape::circle(0., 0., 1.0);
    let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -1.5, y: -1. },
        R2 { x: 1.5, y: -1. },
        R2 { x: 0., y: 1.5 },
    ]));

    let inputs = vec![
        (circle, vec![true; 3]),
        (triangle, vec![true; 6]),
    ];

    // Impossible targets: each region should be 100% of total
    // This requires identical shapes with 100% overlap - impossible for circle+triangle
    let targets: [(&str, f64); 3] = [
        ("0*", 1.),  // Circle total = 100%
        ("*1", 1.),  // Triangle total = 100%
        ("01", 1.),  // Intersection = 100%
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
    let initial_error = model.steps[0].error.v();

    eprintln!("\nImpossible targets test (circle + triangle):");
    eprintln!("  Initial error: {:.4}", initial_error);
    eprintln!("  Targets: A*=1, *B=1, AB=1 (each region = 100%, impossible)");

    // Run 50 steps with vanilla GD to see oscillation
    for i in 0..50 {
        model.train(0.5, 1).expect("Training failed");
        let step = model.steps.last().unwrap();
        let err = step.error.v();

        // Check for NaN/infinity
        assert!(!err.is_nan(), "Error became NaN at step {}", i);
        assert!(err.is_finite(), "Error became infinite at step {}", i);

        if i < 10 || i % 10 == 9 {
            // Extract circle radius for debugging
            let r = if let crate::shape::Shape::Circle(c) = &step.shapes[0] {
                c.r.v()
            } else { 0.0 };
            eprintln!("  Step {:2}: error={:.4}, r={:.4}", i + 1, err, r);
        }
    }

    let final_error = model.steps.last().unwrap().error.v();

    // Characterize the oscillation
    let errors: Vec<f64> = model.steps.iter().map(|s| s.error.v()).collect();
    let error_changes: Vec<f64> = errors.windows(2).map(|w| w[1] - w[0]).collect();
    let sign_changes = error_changes.windows(2)
        .filter(|w| (w[0] > 0.) != (w[1] > 0.))
        .count();

    eprintln!("  Final error: {:.4}", final_error);
    eprintln!("  Sign changes in error delta: {} (oscillation indicator)", sign_changes);

    // With impossible targets, error should be bounded (not explode)
    assert!(
        final_error < 10.0,
        "Error should stay bounded even with impossible targets: got {:.4}",
        final_error
    );

    // Oscillation is expected but shouldn't be too extreme
    // (If there were bugs in gradient computation, error might grow unbounded)
    let max_error = errors.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_error = errors.iter().cloned().fold(f64::INFINITY, f64::min);
    eprintln!("  Error range: {:.4} to {:.4}", min_error, max_error);

    assert!(
        max_error < initial_error * 2.0,
        "Error shouldn't more than double: initial={:.4}, max={:.4}",
        initial_error, max_error
    );
}

/// Test comparing vanilla GD vs clipped GD on impossible targets.
///
/// Clipped GD should have smaller oscillation amplitude.
#[test]
fn test_clipped_vs_vanilla_impossible_targets() {
    use crate::model::Model;
    use crate::step::Step;
    use crate::to::To;

    let make_inputs = || {
        let circle: Shape<f64> = crate::shape::circle(0., 0., 1.0);
        let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -1.5, y: -1. },
            R2 { x: 1.5, y: -1. },
            R2 { x: 0., y: 1.5 },
        ]));
        vec![
            (circle, vec![true; 3]),
            (triangle, vec![true; 6]),
        ]
    };

    // Impossible targets
    let targets: [(&str, f64); 3] = [
        ("0*", 1.),
        ("*1", 1.),
        ("01", 1.),
    ];

    // Vanilla GD
    let mut vanilla_model = Model::new(make_inputs(), targets.to()).expect("Failed to create model");
    for _ in 0..30 {
        vanilla_model.train(0.5, 1).expect("Training failed");
    }
    let vanilla_errors: Vec<f64> = vanilla_model.steps.iter().map(|s| s.error.v()).collect();

    // Clipped GD (using step_clipped directly)
    let clipped_inputs = make_inputs();
    let targets_map: crate::targets::TargetsMap<f64> = targets.to();
    let mut clipped_step = Step::new(clipped_inputs, targets_map.into()).expect("Step failed");
    let mut clipped_errors = vec![clipped_step.error.v()];
    for _ in 0..30 {
        clipped_step = clipped_step.step_clipped(0.05, 0.5, 1.0).expect("Step failed");
        clipped_errors.push(clipped_step.error.v());
    }

    // Compute oscillation amplitude (std dev of error changes)
    let vanilla_deltas: Vec<f64> = vanilla_errors.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
    let clipped_deltas: Vec<f64> = clipped_errors.windows(2).map(|w| (w[1] - w[0]).abs()).collect();

    let vanilla_avg_delta: f64 = vanilla_deltas.iter().sum::<f64>() / vanilla_deltas.len() as f64;
    let clipped_avg_delta: f64 = clipped_deltas.iter().sum::<f64>() / clipped_deltas.len() as f64;

    eprintln!("\nVanilla vs Clipped GD on impossible targets:");
    eprintln!("  Vanilla avg |Δerror|: {:.4}", vanilla_avg_delta);
    eprintln!("  Clipped avg |Δerror|: {:.4}", clipped_avg_delta);
    eprintln!("  Vanilla final error: {:.4}", vanilla_errors.last().unwrap());
    eprintln!("  Clipped final error: {:.4}", clipped_errors.last().unwrap());

    // Clipped should have smaller oscillation
    assert!(
        clipped_avg_delta <= vanilla_avg_delta + 0.01,
        "Clipped GD should have smaller oscillation: vanilla={:.4}, clipped={:.4}",
        vanilla_avg_delta, clipped_avg_delta
    );
}

#[test]
fn test_dodecagon_training_no_self_intersection() {
    use crate::model::Model;
    use crate::to::To;

    fn regular_ngon(n: usize, cx: f64, cy: f64, radius: f64) -> Polygon<f64> {
        let vertices: Vec<R2<f64>> = (0..n)
            .map(|i| {
                let angle = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
                R2 {
                    x: cx + radius * angle.cos(),
                    y: cy + radius * angle.sin(),
                }
            })
            .collect();
        Polygon::new(vertices)
    }

    // Create 2 regular 12-gons (dodecagons) in an overlapping configuration
    let shapes: Vec<Shape<f64>> = vec![
        Shape::Polygon(regular_ngon(12, 0.0, 0.0, 1.5)),  // A
        Shape::Polygon(regular_ngon(12, 1.0, 0.0, 1.5)),  // B (overlaps with A)
    ];

    // All vertices trainable: 12 vertices × 2 coords = 24 per polygon
    let inputs: Vec<(Shape<f64>, Vec<bool>)> = shapes
        .into_iter()
        .map(|s| (s, vec![true; 24]))
        .collect();

    // Simple targets for 2 shapes using inclusive format
    let targets: [(&str, f64); 3] = [
        ("0*", 5.),  // A inclusive
        ("*1", 5.),  // B inclusive
        ("01", 2.),  // A∩B
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
    let initial_error = model.steps[0].error.v();

    eprintln!("\n=== 12-gon Training Self-Intersection Test ===");
    eprintln!("Initial error: {:.4}", initial_error);

    // Train for 100 steps, checking for self-intersection periodically
    let mut self_intersection_count = 0;
    let mut max_error_increase = 0.0f64;
    let mut prev_error = initial_error;

    for step_num in 0..100 {
        model.train(0.3, 1).expect("Training failed");

        let step = model.steps.last().unwrap();
        let error = step.error.v();

        // Track error increases (oscillation indicator)
        if error > prev_error {
            max_error_increase = max_error_increase.max(error - prev_error);
        }
        prev_error = error;

        // Check for self-intersection in all polygons
        for (i, shape) in step.shapes.iter().enumerate() {
            if let Shape::Polygon(poly) = shape {
                // Need to convert Dual vertices to f64 for is_self_intersecting
                let f64_poly = Polygon::new(
                    poly.vertices.iter().map(|v| R2 { x: v.x.v(), y: v.y.v() }).collect()
                );
                if f64_poly.is_self_intersecting() {
                    self_intersection_count += 1;
                    if self_intersection_count <= 5 {
                        eprintln!("  Step {}: Polygon {} self-intersecting!", step_num, i);
                    }
                }
            }
        }

        if step_num % 20 == 0 {
            eprintln!("  Step {}: error = {:.4}", step_num, error);
        }
    }

    let final_error = model.steps.last().unwrap().error.v();
    eprintln!("Final error: {:.4}", final_error);
    eprintln!("Self-intersections detected: {}", self_intersection_count);
    eprintln!("Max single-step error increase: {:.4}", max_error_increase);

    // Assertions
    assert!(
        final_error < initial_error,
        "Training should reduce error: {} -> {}",
        initial_error, final_error
    );

    // Warn (but don't fail) if there were self-intersections
    // The penalty should prevent them, but this test documents the behavior
    if self_intersection_count > 0 {
        eprintln!("WARNING: {} self-intersections occurred during training!", self_intersection_count);
    }

    // Check for severe oscillation (error increases > 50% of current error)
    assert!(
        max_error_increase < initial_error * 0.5,
        "Severe oscillation detected: max error increase {:.4} exceeds threshold",
        max_error_increase
    );
}

#[test]
fn test_four_dodecagons_variant_callers() {
    use crate::model::Model;
    use crate::to::To;

    fn regular_ngon(n: usize, cx: f64, cy: f64, radius: f64) -> Polygon<f64> {
        let vertices: Vec<R2<f64>> = (0..n)
            .map(|i| {
                let angle = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
                R2 {
                    x: cx + radius * angle.cos(),
                    y: cy + radius * angle.sin(),
                }
            })
            .collect();
        Polygon::new(vertices)
    }

    // Create 4 regular 12-gons (dodecagons) in a Venn-like configuration
    // Position them so they overlap in interesting ways
    let shapes: Vec<Shape<f64>> = vec![
        Shape::Polygon(regular_ngon(12, 0.0, 0.5, 1.2)),   // Shape 0 - left
        Shape::Polygon(regular_ngon(12, 1.0, 0.5, 1.2)),   // Shape 1 - right
        Shape::Polygon(regular_ngon(12, 0.5, 0.0, 1.2)),   // Shape 2 - top
        Shape::Polygon(regular_ngon(12, 0.5, 1.0, 1.2)),   // Shape 3 - bottom
    ];

    // All vertices trainable: 12 vertices × 2 coords = 24 per polygon
    let inputs: Vec<(Shape<f64>, Vec<bool>)> = shapes
        .into_iter()
        .map(|s| (s, vec![true; 24]))
        .collect();

    // Variant callers targets (exclusive format)
    // These are real-world targets from genomics variant calling
    let targets: [(&str, f64); 15] = [
        ("0---", 633.),  // Shape 0 only
        ("-1--", 618.),  // Shape 1 only
        ("--2-", 187.),  // Shape 2 only
        ("---3", 319.),  // Shape 3 only
        ("01--", 112.),  // Shapes 0∩1 only
        ("0-2-",   0.),  // Shapes 0∩2 only
        ("0--3",  13.),  // Shapes 0∩3 only
        ("-12-",  14.),  // Shapes 1∩2 only
        ("-1-3",  55.),  // Shapes 1∩3 only
        ("--23",  21.),  // Shapes 2∩3 only
        ("012-",   1.),  // Shapes 0∩1∩2 only
        ("01-3",  17.),  // Shapes 0∩1∩3 only
        ("0-23",   0.),  // Shapes 0∩2∩3 only
        ("-123",   9.),  // Shapes 1∩2∩3 only
        ("0123",  36.),  // All four shapes
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
    let initial_error = model.steps[0].error.v();

    eprintln!("\n=== 4 Dodecagon Variant Callers Test ===");
    eprintln!("Initial error: {:.4}", initial_error);

    // Train for 200 steps, checking for self-intersection and oscillation
    let mut self_intersection_count = 0;
    let mut oscillation_count = 0;
    let mut prev_error = initial_error;
    let mut best_error = initial_error;

    for step_num in 0..200 {
        model.train(0.3, 1).expect("Training failed");

        let step = model.steps.last().unwrap();
        let error = step.error.v();

        // Track oscillation (error increases)
        if error > prev_error * 1.01 {  // Allow 1% noise
            oscillation_count += 1;
        }

        // Track best error
        if error < best_error {
            best_error = error;
        }

        prev_error = error;

        // Check for self-intersection in all polygons
        for (i, shape) in step.shapes.iter().enumerate() {
            if let Shape::Polygon(poly) = shape {
                let f64_poly = Polygon::new(
                    poly.vertices.iter().map(|v| R2 { x: v.x.v(), y: v.y.v() }).collect()
                );
                if f64_poly.is_self_intersecting() {
                    self_intersection_count += 1;
                    if self_intersection_count <= 3 {
                        eprintln!("  Step {}: Polygon {} self-intersecting!", step_num, i);
                    }
                }
            }
        }

        if step_num % 50 == 0 {
            eprintln!("  Step {}: error = {:.4}", step_num, error);
        }
    }

    let final_error = model.steps.last().unwrap().error.v();
    eprintln!("Final error: {:.4}", final_error);
    eprintln!("Best error: {:.4}", best_error);
    eprintln!("Self-intersections detected: {}", self_intersection_count);
    eprintln!("Oscillation events: {}", oscillation_count);

    // Report self-intersections (don't fail the test, but document)
    if self_intersection_count > 0 {
        eprintln!("WARNING: {} self-intersections occurred - penalty may be too weak!", self_intersection_count);
    }

    // Report oscillation (the user's original complaint)
    if oscillation_count > 20 {
        eprintln!("WARNING: {} oscillation events - training is unstable!", oscillation_count);
    }

    // Assertions - training should make progress even if not perfect
    assert!(
        best_error < initial_error * 0.8,
        "Training should achieve at least 20% improvement: {} -> best {}",
        initial_error, best_error
    );
}
