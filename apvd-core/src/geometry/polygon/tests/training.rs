use super::super::*;
use crate::r2::R2;
use crate::shape::Shape;

#[test]
fn test_polygon_circle_training() {
    use crate::model::Model;
    use crate::to::To;

    // Triangle + circle, train to achieve target intersection
    let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -2., y: -1. },
        R2 { x: 2., y: -1. },
        R2 { x: 0., y: 2. },
    ]));
    let circle: Shape<f64> = crate::shape::circle(0., 0., 1.);

    // Only circle can move (simpler optimization)
    let inputs = vec![
        (triangle, vec![false; 6]),
        (circle, vec![true, true, true]),  // cx, cy, r trainable
    ];

    let targets: [(&str, f64); 3] = [
        ("0*", 3.),   // triangle area
        ("*1", 2.),   // circle area
        ("01", 1.),   // intersection
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
    let initial_error = model.steps[0].error.v();

    // Train for some steps
    model.train(0.5, 50).expect("Training failed");

    let final_error = model.steps.last().unwrap().error.v();

    // Error should decrease
    assert!(
        final_error < initial_error,
        "Training should reduce error: {} -> {}",
        initial_error, final_error
    );
}

#[test]
fn test_polygon_circle_gradient_diagnosis() {
    use crate::model::Model;
    use crate::to::To;

    // Reproduce the apvd scenario: triangle + circle with equal targets
    let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -1., y: -1. },
        R2 { x: 1., y: -1. },
        R2 { x: 0., y: 1. },
    ]));
    let circle: Shape<f64> = crate::shape::circle(0.5, 0., 0.8);

    // Both shapes trainable
    let inputs = vec![
        (triangle, vec![true; 6]),  // 6 polygon coords trainable
        (circle, vec![true, true, true]),  // cx, cy, r trainable
    ];

    // Equal weight targets like in the bug report
    let targets: [(&str, f64); 3] = [
        ("0*", 1.),
        ("*1", 1.),
        ("01", 1.),
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");

    // Print initial state and gradients
    let step0 = &model.steps[0];
    eprintln!("Initial error: {}", step0.error.v());
    eprintln!("Initial gradients (d): {:?}", step0.error.d());
    eprintln!("Gradient magnitudes:");
    let grads = step0.error.d();
    for (i, g) in grads.iter().enumerate() {
        let name = if i < 6 {
            format!("polygon v{}.{}", i / 2, if i % 2 == 0 { "x" } else { "y" })
        } else {
            match i - 6 {
                0 => "circle cx".to_string(),
                1 => "circle cy".to_string(),
                2 => "circle r".to_string(),
                _ => format!("param {}", i),
            }
        };
        eprintln!("  {}: {:.6}", name, g);
    }

    // Train and track error over time
    let initial_error = step0.error.v();
    model.train(0.5, 200).expect("Training failed");
    let final_error = model.steps.last().unwrap().error.v();

    // Print error every 20 steps
    eprintln!("\nError progression:");
    for (i, step) in model.steps.iter().enumerate() {
        if i % 20 == 0 || i == model.steps.len() - 1 {
            eprintln!("  Step {}: error = {:.6}", i, step.error.v());
        }
    }

    eprintln!("\nTraining: {} -> {} ({:.1}% reduction)",
        initial_error, final_error,
        100. * (1. - final_error / initial_error));

    // Check if polygon vertices actually moved
    let final_shapes = &model.steps.last().unwrap().shapes;
    if let Shape::Polygon(final_poly) = &final_shapes[0] {
        eprintln!("\nFinal polygon vertices:");
        for (i, v) in final_poly.vertices.iter().enumerate() {
            eprintln!("  v{}: ({:.4}, {:.4})", i, v.x, v.y);
        }
    }
    if let Shape::Circle(final_circle) = &final_shapes[1] {
        eprintln!("Final circle: c=({:.4}, {:.4}), r={:.4}",
            final_circle.c.x, final_circle.c.y, final_circle.r);
    }

    // Should converge reasonably well for this simple case
    assert!(
        final_error < initial_error * 0.5,
        "Should achieve at least 50% error reduction: {} -> {}",
        initial_error, final_error
    );
}

#[test]
fn test_circle_center_gradients_with_polygon() {
    use crate::model::Model;
    use crate::to::To;

    // Test that circle center gradients are non-zero when interacting with polygon
    // Order: Circle first (A), Polygon second (B) - matches user's apvd setup
    // Circle centered at (0, -0.2) - on x symmetry axis, but offset in y
    let circle: Shape<f64> = crate::shape::circle(0., -0.2, 1.);
    // Triangle symmetric around x=0
    let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -2., y: -1. },
        R2 { x: 2., y: -1. },
        R2 { x: 0., y: 2. },
    ]));

    // Circle trainable, polygon fixed
    let inputs = vec![
        (circle, vec![true, true, true]),  // cx, cy, r trainable
        (triangle, vec![false; 6]),
    ];

    let targets: [(&str, f64); 3] = [
        ("0*", 2.),  // circle area (inclusive)
        ("*1", 3.),  // triangle area (inclusive)
        ("01", 1.),  // intersection
    ];

    let model = Model::new(inputs, targets.to()).expect("Failed to create model");
    let step0 = &model.steps[0];
    let grads = step0.error.d();

    eprintln!("Circle-polygon gradient test (circle first):");
    eprintln!("  cx gradient: {:.6e}", grads[0]);
    eprintln!("  cy gradient: {:.6e}", grads[1]);
    eprintln!("  r  gradient: {:.6e}", grads[2]);

    // cx should be ~0 due to symmetry (triangle is symmetric around x=0, circle on axis)
    // cy should be non-zero (moving circle up/down changes intersection)
    // r should be non-zero (changing radius changes intersection)

    assert!(
        grads[1].abs() > 1e-6,
        "cy gradient should be non-zero, got {:.6e}",
        grads[1]
    );
    assert!(
        grads[2].abs() > 1e-6,
        "r gradient should be non-zero, got {:.6e}",
        grads[2]
    );
}

#[test]
fn test_circle_polygon_multi_step_gradients() {
    use crate::model::Model;
    use crate::to::To;

    // Test gradient stability over multiple steps
    // Matches user's scenario more closely
    let circle: Shape<f64> = crate::shape::circle(0., -0.2, 1.3);
    let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -1.6, y: -1.9 },
        R2 { x: 1.6, y: 0.9 },
        R2 { x: -1.6, y: 0.9 },
    ]));

    // Both trainable like user's scenario
    let inputs = vec![
        (circle, vec![true, true, true]),
        (triangle, vec![true; 6]),
    ];

    let targets: [(&str, f64); 3] = [
        ("0*", 1.),
        ("*1", 1.),
        ("01", 1.),
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");

    eprintln!("\nMulti-step gradient test:");
    for i in 0..40 {
        let step = &model.steps[i];
        let grads = step.error.d();
        if i % 10 == 0 || i >= 30 {
            eprintln!("Step {}: error={:.4}, cx_grad={:.4e}, cy_grad={:.4e}, r_grad={:.4e}",
                i, step.error.v(), grads[0], grads[1], grads[2]);
        }
        if i < 39 {
            model.train(0.5, 1).expect("Training failed");
        }
    }

    // Check that gradients don't collapse to zero
    let final_step = model.steps.last().unwrap();
    let final_grads = final_step.error.d();
    let grad_magnitude = (final_grads[0].powi(2) + final_grads[1].powi(2) + final_grads[2].powi(2)).sqrt();

    eprintln!("Final gradient magnitude: {:.4e}", grad_magnitude);

    // If error is still significant (>0.1), gradients shouldn't be zero
    if final_step.error.v() > 0.1 {
        assert!(
            grad_magnitude > 1e-10,
            "Gradients collapsed to zero while error is still {:.4}",
            final_step.error.v()
        );
    }
}

#[test]
fn test_robust_optimizer_comparison() {
    use crate::model::Model;
    use crate::to::To;

    // Compare vanilla GD, Adam, and robust optimization
    let make_model = || {
        let circle: Shape<f64> = crate::shape::circle(0., -0.2, 1.3);
        let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -1.6, y: -1.9 },
            R2 { x: 1.6, y: 0.9 },
            R2 { x: -1.6, y: 0.9 },
        ]));

        let inputs = vec![
            (circle, vec![true, true, true]),
            (triangle, vec![true; 6]),
        ];

        let targets: [(&str, f64); 3] = [
            ("0*", 1.),
            ("*1", 1.),
            ("01", 1.),
        ];

        Model::new(inputs, targets.to()).expect("Failed to create model")
    };

    // Test 1: Vanilla GD
    let mut vanilla = make_model();
    let vanilla_initial = vanilla.steps[0].error.v();
    vanilla.train(0.5, 100).expect("Vanilla training failed");
    let vanilla_final = vanilla.steps.last().unwrap().error.v();

    // Test 2: Adam
    let mut adam = make_model();
    adam.train_adam(0.1, 100).expect("Adam training failed");
    let adam_final = adam.steps.last().unwrap().error.v();

    // Test 3: Robust
    let mut robust = make_model();
    robust.train_robust(100).expect("Robust training failed");
    let robust_final = robust.steps.last().unwrap().error.v();

    eprintln!("\nOptimizer comparison (100 steps):");
    eprintln!("  Initial error: {:.4}", vanilla_initial);
    eprintln!("  Vanilla GD:    {:.4} ({:.1}% reduction)", vanilla_final, 100. * (1. - vanilla_final / vanilla_initial));
    eprintln!("  Adam:          {:.4} ({:.1}% reduction)", adam_final, 100. * (1. - adam_final / vanilla_initial));
    eprintln!("  Robust:        {:.4} ({:.1}% reduction)", robust_final, 100. * (1. - robust_final / vanilla_initial));

    // Check radius stability for each method
    let check_radius_stability = |model: &Model, name: &str| {
        let radii: Vec<f64> = model.steps.iter().map(|s| {
            if let crate::shape::Shape::Circle(c) = &s.shapes[0] {
                c.r.v()
            } else { 0.0 }
        }).collect();

        let max_change: f64 = radii.windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f64::max);

        eprintln!("  {} max radius change: {:.4}", name, max_change);
        max_change
    };

    let vanilla_stability = check_radius_stability(&vanilla, "Vanilla");
    let _adam_stability = check_radius_stability(&adam, "Adam");
    let robust_stability = check_radius_stability(&robust, "Robust");

    // Robust should have more stable updates
    assert!(
        robust_stability <= vanilla_stability + 0.01,
        "Robust should be at least as stable as vanilla"
    );
}

#[test]
fn test_polygon_circle_adam_optimizer() {
    use crate::model::Model;
    use crate::optimization::adam::AdamConfig;
    use crate::to::To;

    // Same setup as gradient diagnosis test, but using Adam optimizer
    let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -1., y: -1. },
        R2 { x: 1., y: -1. },
        R2 { x: 0., y: 1. },
    ]));
    let circle: Shape<f64> = crate::shape::circle(0.5, 0., 0.8);

    // Both shapes trainable
    let inputs = vec![
        (triangle, vec![true; 6]),
        (circle, vec![true, true, true]),
    ];

    // Equal weight targets like in the bug report
    let targets: [(&str, f64); 3] = [
        ("0*", 1.),
        ("*1", 1.),
        ("01", 1.),
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");

    let initial_error = model.steps[0].error.v();
    eprintln!("Initial error: {}", initial_error);

    // Train with Adam optimizer using higher learning rate and more aggressive momentum
    // Default Adam beta1=0.9, beta2=0.999; try beta1=0.95 for more momentum
    let adam_config = AdamConfig {
        beta1: 0.95,
        beta2: 0.999,
        epsilon: 1e-8,
    };
    model.train_adam_with_config(0.2, 500, adam_config).expect("Training failed");

    let final_error = model.steps.last().unwrap().error.v();

    // Print error progression
    eprintln!("\nAdam error progression:");
    for (i, step) in model.steps.iter().enumerate() {
        if i % 50 == 0 || i == model.steps.len() - 1 {
            eprintln!("  Step {}: error = {:.6}", i, step.error.v());
        }
    }

    eprintln!("\nAdam training: {} -> {} ({:.1}% reduction)",
        initial_error, final_error,
        100. * (1. - final_error / initial_error));

    // Both vanilla GD and standard Adam plateau at ~0.61 (54% reduction).
    // This appears to be a local minimum in the error landscape.
    // The test verifies Adam works (doesn't diverge/NaN) and achieves
    // comparable results to vanilla GD. Further improvements may require
    // different loss functions or optimization strategies.
    assert!(
        final_error < initial_error * 0.5,
        "Adam should achieve at least 50% error reduction: {} -> {} ({:.1}% reduction)",
        initial_error, final_error,
        100. * (1. - final_error / initial_error)
    );
}

#[test]
fn test_polygon_ellipse_training() {
    use crate::model::Model;
    use crate::to::To;
    use crate::shape::xyrr;

    // Square + ellipse
    let square: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -1., y: -1. },
        R2 { x: 1., y: -1. },
        R2 { x: 1., y: 1. },
        R2 { x: -1., y: 1. },
    ]));
    let ellipse: Shape<f64> = xyrr(0., 0., 1.5, 0.8);

    // Both can move
    let inputs = vec![
        (square, vec![true; 8]),   // 4 vertices × 2
        (ellipse, vec![true; 4]),  // cx, cy, rx, ry
    ];

    let targets: [(&str, f64); 3] = [
        ("0*", 4.),   // square area
        ("*1", 3.),   // ellipse area
        ("01", 2.),   // intersection
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
    let initial_error = model.steps[0].error.v();

    model.train(0.5, 30).expect("Training failed");

    let final_error = model.steps.last().unwrap().error.v();

    assert!(
        final_error < initial_error,
        "Training should reduce error: {} -> {}",
        initial_error, final_error
    );
}

#[test]
fn test_two_triangles_star_of_david() {
    use crate::model::Model;
    use crate::to::To;
    use std::f64::consts::PI;

    // Exact TwoTriangles layout from apvd frontend (Star of David pattern)
    // Triangle 1: pointing up, centered at (0, 0.3), radius 1.2, rotation -π/2
    let r = 1.2;
    let cy1 = 0.3;
    let rot1 = -PI / 2.0;
    let tri1_poly = Polygon::new(vec![
        R2 { x: r * (rot1).cos(), y: cy1 + r * (rot1).sin() },
        R2 { x: r * (rot1 + 2.0 * PI / 3.0).cos(), y: cy1 + r * (rot1 + 2.0 * PI / 3.0).sin() },
        R2 { x: r * (rot1 + 4.0 * PI / 3.0).cos(), y: cy1 + r * (rot1 + 4.0 * PI / 3.0).sin() },
    ]);

    // Triangle 2: pointing down, centered at (0, -0.3), radius 1.2, rotation π/2
    let cy2 = -0.3;
    let rot2 = PI / 2.0;
    let tri2_poly = Polygon::new(vec![
        R2 { x: r * (rot2).cos(), y: cy2 + r * (rot2).sin() },
        R2 { x: r * (rot2 + 2.0 * PI / 3.0).cos(), y: cy2 + r * (rot2 + 2.0 * PI / 3.0).sin() },
        R2 { x: r * (rot2 + 4.0 * PI / 3.0).cos(), y: cy2 + r * (rot2 + 4.0 * PI / 3.0).sin() },
    ]);

    eprintln!("Triangle 1 vertices:");
    for (i, v) in tri1_poly.vertices.iter().enumerate() {
        eprintln!("  v{}: ({:.4}, {:.4})", i, v.x, v.y);
    }
    eprintln!("Triangle 2 vertices:");
    for (i, v) in tri2_poly.vertices.iter().enumerate() {
        eprintln!("  v{}: ({:.4}, {:.4})", i, v.x, v.y);
    }

    // Test basic intersection first
    let intersections = polygon_polygon_intersect(&tri1_poly, &tri2_poly);
    eprintln!("Intersection points: {:?}", intersections);

    let tri1: Shape<f64> = Shape::Polygon(tri1_poly);
    let tri2: Shape<f64> = Shape::Polygon(tri2_poly);

    // Both polygons fixed for now - just test scene building
    let inputs = vec![
        (tri1, vec![false; 6]),
        (tri2, vec![false; 6]),
    ];

    let targets: [(&str, f64); 3] = [
        ("0*", 1.),
        ("*1", 1.),
        ("01", 1.),
    ];

    let model = Model::new(inputs, targets.to());
    assert!(model.is_ok(), "Failed to create model: {:?}", model.err());

    let model = model.unwrap();
    eprintln!("Model created successfully, initial error: {}", model.steps[0].error.v());
}

#[test]
fn test_vertex_on_edge_intersection() {
    use crate::model::Model;
    use crate::to::To;

    // Create a scenario where one polygon's vertex lands on another's edge
    // This is the edge case that can cause boundary successor issues
    let tri1_poly = Polygon::new(vec![
        R2 { x: 0., y: 0. },      // vertex at origin
        R2 { x: 2., y: 0. },
        R2 { x: 1., y: 2. },
    ]);

    // Second triangle positioned so one vertex lands on tri1's edge
    let tri2_poly = Polygon::new(vec![
        R2 { x: 1., y: 0. },      // This vertex lands on tri1's edge (0,0)-(2,0)
        R2 { x: 2., y: 1.5 },
        R2 { x: 0., y: 1.5 },
    ]);

    eprintln!("Vertex-on-edge test:");
    eprintln!("Triangle 1: {:?}", tri1_poly.vertices);
    eprintln!("Triangle 2: {:?}", tri2_poly.vertices);

    let intersections = polygon_polygon_intersect(&tri1_poly, &tri2_poly);
    eprintln!("Intersections: {:?}", intersections);

    let tri1: Shape<f64> = Shape::Polygon(tri1_poly);
    let tri2: Shape<f64> = Shape::Polygon(tri2_poly);

    let inputs = vec![
        (tri1, vec![false; 6]),
        (tri2, vec![false; 6]),
    ];

    let targets: [(&str, f64); 3] = [
        ("0*", 1.),
        ("*1", 1.),
        ("01", 0.5),
    ];

    let model = Model::new(inputs, targets.to());
    if model.is_err() {
        eprintln!("Model creation failed (expected for vertex-on-edge): {:?}", model.err());
    } else {
        eprintln!("Model created successfully, error: {}", model.unwrap().steps[0].error.v());
    }
}

#[test]
fn test_polygon_polygon_training() {
    use crate::model::Model;
    use crate::to::To;

    // Two triangles with no coincident edges
    // tri1: base at y=-1, apex at y=1
    let tri1_poly = Polygon::new(vec![
        R2 { x: -1., y: -1. },
        R2 { x: 1., y: -1. },
        R2 { x: 0., y: 1. },
    ]);
    // tri2: rotated differently, centered higher
    let tri2_poly = Polygon::new(vec![
        R2 { x: -0.5, y: 0.2 },
        R2 { x: 1.3, y: 0.1 },
        R2 { x: 0.4, y: 1.8 },
    ]);

    // Test basic intersection first
    let intersections = polygon_polygon_intersect(&tri1_poly, &tri2_poly);
    eprintln!("intersection points: {:?}", intersections);

    let tri1: Shape<f64> = Shape::Polygon(tri1_poly);
    let tri2: Shape<f64> = Shape::Polygon(tri2_poly);

    // Only second triangle moves
    let inputs = vec![
        (tri1, vec![false; 6]),
        (tri2, vec![true; 6]),
    ];

    let targets: [(&str, f64); 3] = [
        ("0*", 1.),
        ("*1", 1.),
        ("01", 0.3),
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
    let initial_error = model.steps[0].error.v();

    model.train(0.5, 50).expect("Training failed");

    let final_error = model.steps.last().unwrap().error.v();

    assert!(
        final_error < initial_error,
        "Training should reduce error: {} -> {}",
        initial_error, final_error
    );
}
