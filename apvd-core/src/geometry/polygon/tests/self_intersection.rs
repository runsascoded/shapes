use super::super::*;
use crate::dual::Dual;
use crate::r2::R2;

/// Create a self-intersecting "bowtie" quadrilateral (figure-8 shape)
fn bowtie() -> Polygon<f64> {
    // Vertices in order that creates crossing edges
    Polygon::new(vec![
        R2 { x: 0., y: 0. },
        R2 { x: 2., y: 2. },  // crosses with edge 2-3
        R2 { x: 2., y: 0. },
        R2 { x: 0., y: 2. },  // crosses with edge 0-1
    ])
}

/// Create a regular n-gon centered at (cx, cy) with given radius
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

fn triangle() -> Polygon<f64> {
    Polygon::new(vec![
        R2 { x: 0., y: 0. },
        R2 { x: 1., y: 0. },
        R2 { x: 0.5, y: 1. },
    ])
}

fn square() -> Polygon<f64> {
    Polygon::new(vec![
        R2 { x: 0., y: 0. },
        R2 { x: 1., y: 0. },
        R2 { x: 1., y: 1. },
        R2 { x: 0., y: 1. },
    ])
}

#[test]
fn test_is_self_intersecting_simple() {
    // Convex shapes should not self-intersect
    assert!(!triangle().is_self_intersecting(), "Triangle should not self-intersect");
    assert!(!square().is_self_intersecting(), "Square should not self-intersect");

    // Regular n-gons should not self-intersect
    for n in 5..=12 {
        let ngon = regular_ngon(n, 0., 0., 1.);
        assert!(!ngon.is_self_intersecting(), "{}-gon should not self-intersect", n);
    }

    // Bowtie (figure-8) should self-intersect
    assert!(bowtie().is_self_intersecting(), "Bowtie should self-intersect");
}

#[test]
fn test_self_intersection_penalty_nonzero_for_bowtie() {
    let bow = bowtie();
    let penalty = bow.self_intersection_penalty();
    assert!(penalty > 0., "Bowtie should have positive self-intersection penalty, got {}", penalty);

    // Regular shapes should have zero penalty
    assert_eq!(square().self_intersection_penalty(), 0., "Square should have zero penalty");
    assert_eq!(regular_ngon(12, 0., 0., 1.).self_intersection_penalty(), 0., "12-gon should have zero penalty");
}

#[test]
fn test_self_intersection_penalty_dual_produces_gradients() {
    // Create a nearly self-intersecting quadrilateral that can become self-intersecting
    // by moving one vertex slightly
    let almost_bowtie: Polygon<Dual> = Polygon::new(vec![
        R2 { x: Dual::new(0., vec![1., 0., 0., 0., 0., 0., 0., 0.]), y: Dual::new(0., vec![0., 1., 0., 0., 0., 0., 0., 0.]) },
        R2 { x: Dual::new(1., vec![0., 0., 1., 0., 0., 0., 0., 0.]), y: Dual::new(0., vec![0., 0., 0., 1., 0., 0., 0., 0.]) },
        R2 { x: Dual::new(0.5, vec![0., 0., 0., 0., 1., 0., 0., 0.]), y: Dual::new(1., vec![0., 0., 0., 0., 0., 1., 0., 0.]) },
        R2 { x: Dual::new(0.5, vec![0., 0., 0., 0., 0., 0., 1., 0.]), y: Dual::new(-0.1, vec![0., 0., 0., 0., 0., 0., 0., 1.]) },  // below x-axis, causes self-intersection
    ]);

    let penalty = almost_bowtie.self_intersection_penalty_dual();

    // Should have positive penalty
    assert!(penalty.v() > 0., "Self-intersecting quad should have positive penalty, got {}", penalty.v());

    // Should have non-zero gradients
    let grads = penalty.d();
    let grad_magnitude: f64 = grads.iter().map(|g| g.powi(2)).sum::<f64>().sqrt();
    assert!(grad_magnitude > 0., "Should have non-zero gradients for self-intersection penalty");

    eprintln!("Self-intersection penalty: {}", penalty.v());
    eprintln!("Gradient magnitude: {}", grad_magnitude);
}
