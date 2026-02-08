use super::super::*;
use crate::r2::R2;
use crate::shape::Shape;

/// Test that two circles converge to achievable targets.
///
/// This is the simplest case: two circles can achieve any overlap from 0 to min(area_A, area_B).
/// We set targets based on actual computed areas, perturb, and verify convergence.
#[test]
fn test_two_circles_convergence() {
    use crate::model::Model;
    use crate::step::Step;
    use crate::to::To;

    // Two unit circles, slightly overlapping
    let circle_a: Shape<f64> = crate::shape::circle(-0.3, 0., 1.0);
    let circle_b: Shape<f64> = crate::shape::circle(0.3, 0., 1.0);

    // First, compute what the actual areas are at this "solution" position
    let solution_inputs = vec![
        (circle_a.clone(), vec![true; 3]),
        (circle_b.clone(), vec![true; 3]),
    ];

    // Use dummy targets to compute actual areas
    let dummy_targets: [(&str, f64); 3] = [("0*", 1.), ("*1", 1.), ("01", 0.)];
    let targets_map: crate::targets::TargetsMap<f64> = dummy_targets.to();
    let solution_step = Step::new(solution_inputs, targets_map.into()).expect("Solution step failed");

    // Extract actual areas from the solution
    let area_a_total = solution_step.errors.get("0*").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();
    let area_b_total = solution_step.errors.get("*1").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();
    let area_intersection = solution_step.errors.get("01").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();

    eprintln!("Solution areas: A_total={:.4}, B_total={:.4}, intersection={:.4}",
        area_a_total, area_b_total, area_intersection);

    // Now perturb the shapes and train back to the solution
    let perturbed_a: Shape<f64> = crate::shape::circle(-0.5, 0.2, 1.1);  // Moved and resized
    let perturbed_b: Shape<f64> = crate::shape::circle(0.5, -0.1, 0.9); // Moved and resized

    let inputs = vec![
        (perturbed_a, vec![true; 3]),
        (perturbed_b, vec![true; 3]),
    ];

    // Targets based on solution areas (these are achievable!)
    let targets: [(&str, f64); 3] = [
        ("0*", area_a_total),
        ("*1", area_b_total),
        ("01", area_intersection),
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
    let initial_error = model.steps[0].error.v();

    eprintln!("\nTwo circles convergence test:");
    eprintln!("  Initial error: {:.6}", initial_error);

    // Train with robust optimizer
    model.train_robust(200).expect("Training failed");

    let final_error = model.steps.last().unwrap().error.v();
    let reduction_pct = 100. * (1. - final_error / initial_error);

    eprintln!("  Final error: {:.6} ({:.1}% reduction)", final_error, reduction_pct);
    eprintln!("  Steps taken: {}", model.steps.len());

    // Print error progression
    for (i, step) in model.steps.iter().enumerate() {
        if i % 25 == 0 || i == model.steps.len() - 1 {
            eprintln!("  Step {:3}: error = {:.6}", i, step.error.v());
        }
    }

    // With achievable targets, we should converge to very low error
    assert!(
        final_error < 0.1,
        "Should converge to near-zero error with achievable targets: got {:.4}",
        final_error
    );
    assert!(
        reduction_pct > 90.,
        "Should achieve >90% error reduction: got {:.1}%",
        reduction_pct
    );
}

/// Test that a triangle and circle converge to achievable targets.
#[test]
fn test_triangle_circle_convergence() {
    use crate::model::Model;
    use crate::step::Step;
    use crate::to::To;

    // Triangle and circle in a "solution" overlapping position
    let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -1., y: -0.8 },
        R2 { x: 1., y: -0.8 },
        R2 { x: 0., y: 1.2 },
    ]));
    let circle: Shape<f64> = crate::shape::circle(0., 0., 0.8);

    // Compute actual areas at solution
    let solution_inputs = vec![
        (triangle.clone(), vec![true; 6]),
        (circle.clone(), vec![true; 3]),
    ];
    let dummy_targets: [(&str, f64); 3] = [("0*", 1.), ("*1", 1.), ("01", 0.)];
    let targets_map: crate::targets::TargetsMap<f64> = dummy_targets.to();
    let solution_step = Step::new(solution_inputs, targets_map.into()).expect("Solution step failed");

    let area_a_total = solution_step.errors.get("0*").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();
    let area_b_total = solution_step.errors.get("*1").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();
    let area_intersection = solution_step.errors.get("01").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();

    eprintln!("Triangle-circle solution areas: A_total={:.4}, B_total={:.4}, intersection={:.4}",
        area_a_total, area_b_total, area_intersection);

    // Perturb: move triangle up and circle to the side
    let perturbed_triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -1.2, y: -0.5 },
        R2 { x: 1.2, y: -0.5 },
        R2 { x: 0., y: 1.5 },
    ]));
    let perturbed_circle: Shape<f64> = crate::shape::circle(0.3, 0.2, 0.9);

    let inputs = vec![
        (perturbed_triangle, vec![true; 6]),
        (perturbed_circle, vec![true; 3]),
    ];

    let targets: [(&str, f64); 3] = [
        ("0*", area_a_total),
        ("*1", area_b_total),
        ("01", area_intersection),
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
    let initial_error = model.steps[0].error.v();

    eprintln!("\nTriangle-circle convergence test:");
    eprintln!("  Initial error: {:.6}", initial_error);

    model.train_robust(300).expect("Training failed");

    let final_error = model.steps.last().unwrap().error.v();
    let reduction_pct = 100. * (1. - final_error / initial_error);

    eprintln!("  Final error: {:.6} ({:.1}% reduction)", final_error, reduction_pct);

    for (i, step) in model.steps.iter().enumerate() {
        if i % 50 == 0 || i == model.steps.len() - 1 {
            eprintln!("  Step {:3}: error = {:.6}", i, step.error.v());
        }
    }

    assert!(
        final_error < 0.15,
        "Should converge to low error with achievable targets: got {:.4}",
        final_error
    );
}

/// Test that two triangles converge to achievable targets.
#[test]
fn test_two_triangles_convergence() {
    use crate::model::Model;
    use crate::step::Step;
    use crate::to::To;

    // Two triangles in an overlapping "solution" position
    let triangle_a: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -1., y: -0.5 },
        R2 { x: 1., y: -0.5 },
        R2 { x: 0., y: 1. },
    ]));
    let triangle_b: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -0.8, y: -0.3 },
        R2 { x: 0.8, y: -0.3 },
        R2 { x: 0., y: 1.2 },
    ]));

    // Compute actual areas at solution
    let solution_inputs = vec![
        (triangle_a.clone(), vec![true; 6]),
        (triangle_b.clone(), vec![true; 6]),
    ];
    let dummy_targets: [(&str, f64); 3] = [("0*", 1.), ("*1", 1.), ("01", 0.)];
    let targets_map: crate::targets::TargetsMap<f64> = dummy_targets.to();
    let solution_step = Step::new(solution_inputs, targets_map.into()).expect("Solution step failed");

    let area_a_total = solution_step.errors.get("0*").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();
    let area_b_total = solution_step.errors.get("*1").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();
    let area_intersection = solution_step.errors.get("01").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();

    eprintln!("Two triangles solution areas: A_total={:.4}, B_total={:.4}, intersection={:.4}",
        area_a_total, area_b_total, area_intersection);

    // Perturb both triangles
    let perturbed_a: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -1.3, y: -0.7 },
        R2 { x: 0.9, y: -0.4 },
        R2 { x: -0.1, y: 1.2 },
    ]));
    let perturbed_b: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -0.6, y: -0.1 },
        R2 { x: 1.0, y: -0.4 },
        R2 { x: 0.2, y: 1.0 },
    ]));

    let inputs = vec![
        (perturbed_a, vec![true; 6]),
        (perturbed_b, vec![true; 6]),
    ];

    let targets: [(&str, f64); 3] = [
        ("0*", area_a_total),
        ("*1", area_b_total),
        ("01", area_intersection),
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
    let initial_error = model.steps[0].error.v();

    eprintln!("\nTwo triangles convergence test:");
    eprintln!("  Initial error: {:.6}", initial_error);

    model.train_robust(300).expect("Training failed");

    let final_error = model.steps.last().unwrap().error.v();
    let reduction_pct = 100. * (1. - final_error / initial_error);

    eprintln!("  Final error: {:.6} ({:.1}% reduction)", final_error, reduction_pct);

    for (i, step) in model.steps.iter().enumerate() {
        if i % 50 == 0 || i == model.steps.len() - 1 {
            eprintln!("  Step {:3}: error = {:.6}", i, step.error.v());
        }
    }

    assert!(
        final_error < 0.2,
        "Should converge to low error with achievable targets: got {:.4}",
        final_error
    );
}

/// Test with very simple achievable targets: complete disjointness.
/// Two shapes that don't need to overlap at all - simplest case.
#[test]
fn test_disjoint_targets_convergence() {
    use crate::model::Model;
    use crate::to::To;

    // Two circles that should become disjoint
    let circle_a: Shape<f64> = crate::shape::circle(-0.5, 0., 1.0);
    let circle_b: Shape<f64> = crate::shape::circle(0.5, 0., 1.0);

    let inputs = vec![
        (circle_a, vec![true; 3]),
        (circle_b, vec![true; 3]),
    ];

    // Targets: each circle has area π, no overlap
    // Using fractions that sum to 1 (the optimization uses fractions, not absolute areas)
    let targets: [(&str, f64); 3] = [
        ("0*", 1.),   // Circle A total
        ("*1", 1.),   // Circle B total
        ("01", 0.),   // No intersection - this is achievable by separating the circles!
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
    let initial_error = model.steps[0].error.v();

    eprintln!("\nDisjoint targets convergence test:");
    eprintln!("  Initial error: {:.6}", initial_error);

    model.train_robust(200).expect("Training failed");

    let final_error = model.steps.last().unwrap().error.v();
    let reduction_pct = 100. * (1. - final_error / initial_error);

    eprintln!("  Final error: {:.6} ({:.1}% reduction)", final_error, reduction_pct);

    for (i, step) in model.steps.iter().enumerate() {
        if i % 25 == 0 || i == model.steps.len() - 1 {
            eprintln!("  Step {:3}: error = {:.6}", i, step.error.v());
        }
    }

    // Disjoint is easy - shapes just need to separate
    assert!(
        final_error < 0.05,
        "Should converge to near-zero for disjoint targets: got {:.4}",
        final_error
    );
}
