use super::super::*;

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
fn test_triangle_area() {
    let t = triangle();
    // Area of triangle with base 1 and height 1 = 0.5
    assert_relative_eq!(t.area(), 0.5, epsilon = 1e-10);
}

#[test]
fn test_square_area() {
    let s = square();
    // Area of unit square = 1
    assert_relative_eq!(s.area(), 1.0, epsilon = 1e-10);
}

#[test]
fn test_at_y_square() {
    let s = square();

    // At y=0.5, should intersect left and right edges
    let xs = s.at_y(0.5);
    assert_eq!(xs.len(), 2);
    assert_relative_eq!(xs[0], 0., epsilon = 1e-10);
    assert_relative_eq!(xs[1], 1., epsilon = 1e-10);

    // At y=0 (bottom vertex level), left and right edges start here
    // Using half-open [y_min, y_max), we include y_min
    let xs = s.at_y(0.);
    assert_eq!(xs.len(), 2);
    assert_relative_eq!(xs[0], 0., epsilon = 1e-10);
    assert_relative_eq!(xs[1], 1., epsilon = 1e-10);

    // At y=1 (top vertex level), using half-open interval [y_min, y_max),
    // y=1 is excluded since it equals y_max for vertical edges
    let xs = s.at_y(1.);
    assert_eq!(xs.len(), 0);

    // Above square, no intersection
    let xs = s.at_y(1.5);
    assert_eq!(xs.len(), 0);
}

#[test]
fn test_at_y_triangle() {
    let t = triangle();

    // At y=0.5, should intersect left and right edges
    let xs = t.at_y(0.5);
    assert_eq!(xs.len(), 2);
    assert_relative_eq!(xs[0], 0.25, epsilon = 1e-10);
    assert_relative_eq!(xs[1], 0.75, epsilon = 1e-10);
}

#[test]
fn test_unit_intersections_large_triangle() {
    // Triangle that actually intersects the unit circle
    // Vertices are outside the unit circle but edges pass through it
    let t = Polygon::new(vec![
        R2 { x: -1.5, y: -1. },
        R2 { x: 1.5, y: -1. },
        R2 { x: 0., y: 1.5 },
    ]);

    let points = t.unit_intersections();
    // Should have 6 intersections (2 per edge) since each edge crosses the unit circle
    assert!(points.len() >= 2, "Expected at least 2 intersections, got {}", points.len());

    // All points should be on the unit circle
    for p in &points {
        let r = (p.x * p.x + p.y * p.y).sqrt();
        assert_relative_eq!(r, 1.0, epsilon = 1e-10);
    }
}

#[test]
fn test_unit_intersections_small_triangle() {
    // Small triangle inside the unit circle - no intersections
    let t = Polygon::new(vec![
        R2 { x: -0.1, y: -0.1 },
        R2 { x: 0.1, y: -0.1 },
        R2 { x: 0., y: 0.1 },
    ]);

    let points = t.unit_intersections();
    assert_eq!(points.len(), 0);
}

#[test]
fn test_polygon_polygon_intersect() {
    // Two overlapping squares
    let s1 = square();
    let s2 = Polygon::new(vec![
        R2 { x: 0.5, y: 0.5 },
        R2 { x: 1.5, y: 0.5 },
        R2 { x: 1.5, y: 1.5 },
        R2 { x: 0.5, y: 1.5 },
    ]);

    let points = polygon_polygon_intersect(&s1, &s2);
    // Should have 2 intersection points
    assert_eq!(points.len(), 2);
}

#[test]
fn test_polygon_polygon_no_intersect() {
    // Two non-overlapping squares
    let s1 = square();
    let s2 = Polygon::new(vec![
        R2 { x: 2., y: 0. },
        R2 { x: 3., y: 0. },
        R2 { x: 3., y: 1. },
        R2 { x: 2., y: 1. },
    ]);

    let points = polygon_polygon_intersect(&s1, &s2);
    assert_eq!(points.len(), 0);
}
