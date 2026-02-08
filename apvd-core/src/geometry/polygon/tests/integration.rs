use super::super::*;
use crate::r2::R2;
use crate::shape::Shape;
use crate::transform::Transform::Translate;

fn triangle() -> Polygon<f64> {
    Polygon::new(vec![
        R2 { x: 0., y: 0. },
        R2 { x: 1., y: 0. },
        R2 { x: 0.5, y: 1. },
    ])
}

#[test]
fn test_center() {
    let t = triangle();
    let c = t.center();
    assert_relative_eq!(c.x, 0.5, epsilon = 1e-10);
    assert_relative_eq!(c.y, 1. / 3., epsilon = 1e-10);
}

#[test]
fn test_names_and_vals() {
    let t = triangle();
    let names = t.names();
    assert_eq!(names, vec!["v0.x", "v0.y", "v1.x", "v1.y", "v2.x", "v2.y"]);

    let vals = t.vals();
    assert_eq!(vals, vec![0., 0., 1., 0., 0.5, 1.]);
}

#[test]
fn test_transform_translate() {
    use crate::transform::CanTransform;
    let t = triangle();
    let translated = t.transform(&Translate(R2 { x: 1., y: 2. }));
    match translated {
        Shape::Polygon(p) => {
            assert_relative_eq!(p.vertices[0].x, 1., epsilon = 1e-10);
            assert_relative_eq!(p.vertices[0].y, 2., epsilon = 1e-10);
        }
        _ => panic!("Expected Polygon"),
    }
}

#[test]
fn test_transform_scale() {
    use crate::transform::{CanTransform, Transform::Scale};
    let t = triangle();
    let scaled = t.transform(&Scale(2.));
    match scaled {
        Shape::Polygon(p) => {
            assert_relative_eq!(p.vertices[1].x, 2., epsilon = 1e-10);
            assert_relative_eq!(p.vertices[2].y, 2., epsilon = 1e-10);
        }
        _ => panic!("Expected Polygon"),
    }
}

#[test]
fn test_display() {
    let t = triangle();
    let s = format!("{}", t);
    assert!(s.starts_with("Polygon["));
}

#[test]
fn test_polygon_xyrrt_intersect() {
    use crate::ellipses::xyrrt::XYRRT;
    use crate::intersect::Intersect;

    // Triangle that intersects a rotated ellipse
    let polygon = Polygon::new(vec![
        R2 { x: -2., y: 0. },
        R2 { x: 2., y: 0. },
        R2 { x: 0., y: 2. },
    ]);

    // Rotated ellipse at origin
    let xyrrt: Shape<f64> = Shape::XYRRT(XYRRT {
        c: R2 { x: 0., y: 0.5 },
        r: R2 { x: 1.5, y: 0.8 },
        t: 0.3,
    });

    let polygon_shape: Shape<f64> = Shape::Polygon(polygon);

    // Test both orderings work and give same results
    let points1 = polygon_shape.intersect(&xyrrt);
    let points2 = xyrrt.intersect(&polygon_shape);

    assert!(!points1.is_empty(), "Expected intersections, got none");
    assert_eq!(points1.len(), points2.len(), "Both orderings should give same number of points");

    // Verify points are approximately the same (order may differ)
    for p1 in &points1 {
        let found = points2.iter().any(|p2| {
            (p1.x - p2.x).abs() < 1e-10 && (p1.y - p2.y).abs() < 1e-10
        });
        assert!(found, "Point {:?} not found in reverse intersection", p1);
    }
}

#[test]
fn test_polygon_circle_scene() {
    use crate::model::Model;
    use crate::to::To;

    // This tests the full scene building with regions - requires theta computation
    let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -1.5, y: -1. },
        R2 { x: 1.5, y: -1. },
        R2 { x: 0., y: 1.5 },
    ]));
    let circle: Shape<f64> = crate::shape::circle(0., 0., 1.);

    // InputSpec is (Shape<f64>, Vec<bool>) - bools indicate which coords are trainable
    let inputs = vec![
        (triangle, vec![true; 6]),  // 6 coords for triangle (3 vertices × 2)
        (circle, vec![true; 3]),    // 3 coords for circle (cx, cy, r)
    ];

    // Target keys using inclusive patterns like existing tests
    let targets: [(&str, f64); 3] = [
        ("0*", 1.),  // triangle (inclusive)
        ("*1", 1.),  // circle (inclusive)
        ("01", 0.5), // intersection
    ];

    // This will try to build regions, which requires theta computation on polygons
    let model = Model::new(inputs, targets.to());
    assert!(model.is_ok(), "Failed to build model: {:?}", model.err());
}

#[test]
fn test_concave_polygon() {
    use crate::model::Model;
    use crate::to::To;

    // L-shaped polygon (concave)
    let l_shape: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: 0., y: 0. },
        R2 { x: 2., y: 0. },
        R2 { x: 2., y: 1. },
        R2 { x: 1., y: 1. },
        R2 { x: 1., y: 2. },
        R2 { x: 0., y: 2. },
    ]));
    let circle: Shape<f64> = crate::shape::circle(1., 1., 0.8);

    let inputs = vec![
        (l_shape, vec![false; 12]),  // 6 vertices × 2
        (circle, vec![true; 3]),
    ];

    let targets: [(&str, f64); 3] = [
        ("0*", 3.),   // L area = 2×2 - 1×1 = 3
        ("*1", 2.),
        ("01", 1.),
    ];

    let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
    let initial_error = model.steps[0].error.v();

    model.train(0.5, 30).expect("Training failed");

    let final_error = model.steps.last().unwrap().error.v();

    assert!(
        final_error <= initial_error,
        "Training should not increase error: {} -> {}",
        initial_error, final_error
    );
}

#[test]
fn test_three_shape_mixed() {
    use crate::model::Model;
    use crate::to::To;

    // Triangle + circle + ellipse - triangle much larger to ensure partial overlap
    let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
        R2 { x: -3., y: -2. },
        R2 { x: 3., y: -2. },
        R2 { x: 0., y: 3. },
    ]));
    let circle: Shape<f64> = crate::shape::circle(-0.5, 0., 0.5);
    let ellipse: Shape<f64> = crate::shape::xyrr(0.5, 0., 0.4, 0.6);

    let inputs = vec![
        (triangle, vec![false; 6]),  // Fixed
        (circle, vec![false; 3]),    // Fixed (avoid training issues for now)
        (ellipse, vec![false; 4]),   // Fixed
    ];

    // 3 shapes = 7 non-empty regions (2^3 - 1)
    let targets: [(&str, f64); 7] = [
        ("0**", 7.5),  // triangle
        ("*1*", 0.8),  // circle
        ("**2", 0.75), // ellipse
        ("01*", 0.5),  // triangle ∩ circle
        ("0*2", 0.5),  // triangle ∩ ellipse
        ("*12", 0.2),  // circle ∩ ellipse
        ("012", 0.1),  // all three
    ];

    // Just verify model builds without panic
    let model = Model::new(inputs, targets.to());
    assert!(model.is_ok(), "Failed to create model: {:?}", model.err());
}
