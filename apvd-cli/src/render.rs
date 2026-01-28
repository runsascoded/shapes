//! SVG rendering for shapes and training traces.

use std::fmt::Write;

use apvd_core::shape::Shape;
use apvd_core::D;

/// SVG rendering configuration
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Canvas width in pixels
    pub width: f64,
    /// Canvas height in pixels
    pub height: f64,
    /// Padding around shapes (fraction of canvas)
    pub padding: f64,
    /// Stroke width for shape outlines
    pub stroke_width: f64,
    /// Whether to fill shapes
    pub fill: bool,
    /// Fill opacity (0.0 - 1.0)
    pub fill_opacity: f64,
    /// Whether to show shape labels
    pub show_labels: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            padding: 0.1,
            stroke_width: 2.0,
            fill: true,
            fill_opacity: 0.3,
            show_labels: true,
        }
    }
}

/// Color palette for shapes
const COLORS: &[&str] = &[
    "#e41a1c", // red
    "#377eb8", // blue
    "#4daf4a", // green
    "#984ea3", // purple
    "#ff7f00", // orange
    "#ffff33", // yellow
    "#a65628", // brown
    "#f781bf", // pink
];

/// Render shapes to SVG string
pub fn render_svg(shapes: &[Shape<D>], config: &RenderConfig) -> String {
    let shapes_f64: Vec<_> = shapes.iter().map(|s| s.v()).collect();

    // Compute bounding box
    let (min_x, max_x, min_y, max_y) = compute_bounds(&shapes_f64);

    // Add padding
    let width = max_x - min_x;
    let height = max_y - min_y;
    let pad_x = width * config.padding;
    let pad_y = height * config.padding;

    let view_min_x = min_x - pad_x;
    let view_min_y = min_y - pad_y;
    let view_width = width + 2.0 * pad_x;
    let view_height = height + 2.0 * pad_y;

    let mut svg = String::new();

    // SVG header
    writeln!(
        &mut svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="{} {} {} {}">"#,
        config.width, config.height, view_min_x, view_min_y, view_width, view_height
    ).unwrap();

    // Background
    writeln!(
        &mut svg,
        r#"  <rect x="{}" y="{}" width="{}" height="{}" fill="white"/>"#,
        view_min_x, view_min_y, view_width, view_height
    ).unwrap();

    // Render each shape
    for (idx, shape) in shapes_f64.iter().enumerate() {
        let color = COLORS[idx % COLORS.len()];
        let fill = if config.fill {
            format!(r#"fill="{}" fill-opacity="{}""#, color, config.fill_opacity)
        } else {
            r#"fill="none""#.to_string()
        };

        match shape {
            apvd_core::shape::Shape::Circle(c) => {
                writeln!(
                    &mut svg,
                    r#"  <circle cx="{}" cy="{}" r="{}" {} stroke="{}" stroke-width="{}"/>"#,
                    c.c.x, c.c.y, c.r, fill, color, config.stroke_width
                ).unwrap();

                if config.show_labels {
                    writeln!(
                        &mut svg,
                        r#"  <text x="{}" y="{}" font-size="{}" text-anchor="middle" fill="{}">{}</text>"#,
                        c.c.x, c.c.y, config.stroke_width * 8.0, color, idx
                    ).unwrap();
                }
            }
            apvd_core::shape::Shape::XYRR(e) => {
                // Axis-aligned ellipse
                writeln!(
                    &mut svg,
                    r#"  <ellipse cx="{}" cy="{}" rx="{}" ry="{}" {} stroke="{}" stroke-width="{}"/>"#,
                    e.c.x, e.c.y, e.r.x, e.r.y, fill, color, config.stroke_width
                ).unwrap();

                if config.show_labels {
                    writeln!(
                        &mut svg,
                        r#"  <text x="{}" y="{}" font-size="{}" text-anchor="middle" fill="{}">{}</text>"#,
                        e.c.x, e.c.y, config.stroke_width * 8.0, color, idx
                    ).unwrap();
                }
            }
            apvd_core::shape::Shape::XYRRT(e) => {
                // Rotated ellipse - use transform
                let angle_deg = e.t.to_degrees();
                writeln!(
                    &mut svg,
                    r#"  <ellipse cx="{}" cy="{}" rx="{}" ry="{}" transform="rotate({} {} {})" {} stroke="{}" stroke-width="{}"/>"#,
                    e.c.x, e.c.y, e.r.x, e.r.y, angle_deg, e.c.x, e.c.y, fill, color, config.stroke_width
                ).unwrap();

                if config.show_labels {
                    writeln!(
                        &mut svg,
                        r#"  <text x="{}" y="{}" font-size="{}" text-anchor="middle" fill="{}">{}</text>"#,
                        e.c.x, e.c.y, config.stroke_width * 8.0, color, idx
                    ).unwrap();
                }
            }
            apvd_core::shape::Shape::Polygon(p) => {
                // Polygon as path
                if p.vertices.is_empty() {
                    continue;
                }

                let mut path = format!("M {} {}", p.vertices[0].x, p.vertices[0].y);
                for v in &p.vertices[1..] {
                    write!(&mut path, " L {} {}", v.x, v.y).unwrap();
                }
                path.push_str(" Z");

                writeln!(
                    &mut svg,
                    r#"  <path d="{}" {} stroke="{}" stroke-width="{}"/>"#,
                    path, fill, color, config.stroke_width
                ).unwrap();

                // Label at centroid
                if config.show_labels {
                    let cx: f64 = p.vertices.iter().map(|v| v.x).sum::<f64>() / p.vertices.len() as f64;
                    let cy: f64 = p.vertices.iter().map(|v| v.y).sum::<f64>() / p.vertices.len() as f64;
                    writeln!(
                        &mut svg,
                        r#"  <text x="{}" y="{}" font-size="{}" text-anchor="middle" fill="{}">{}</text>"#,
                        cx, cy, config.stroke_width * 8.0, color, idx
                    ).unwrap();
                }
            }
        }
    }

    // Close SVG
    writeln!(&mut svg, "</svg>").unwrap();

    svg
}

/// Compute bounding box for shapes
fn compute_bounds(shapes: &[apvd_core::shape::Shape<f64>]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for shape in shapes {
        match shape {
            apvd_core::shape::Shape::Circle(c) => {
                min_x = min_x.min(c.c.x - c.r);
                max_x = max_x.max(c.c.x + c.r);
                min_y = min_y.min(c.c.y - c.r);
                max_y = max_y.max(c.c.y + c.r);
            }
            apvd_core::shape::Shape::XYRR(e) => {
                min_x = min_x.min(e.c.x - e.r.x);
                max_x = max_x.max(e.c.x + e.r.x);
                min_y = min_y.min(e.c.y - e.r.y);
                max_y = max_y.max(e.c.y + e.r.y);
            }
            apvd_core::shape::Shape::XYRRT(e) => {
                // Conservative bounds for rotated ellipse
                let max_r = e.r.x.max(e.r.y);
                min_x = min_x.min(e.c.x - max_r);
                max_x = max_x.max(e.c.x + max_r);
                min_y = min_y.min(e.c.y - max_r);
                max_y = max_y.max(e.c.y + max_r);
            }
            apvd_core::shape::Shape::Polygon(p) => {
                for v in &p.vertices {
                    min_x = min_x.min(v.x);
                    max_x = max_x.max(v.x);
                    min_y = min_y.min(v.y);
                    max_y = max_y.max(v.y);
                }
            }
        }
    }

    // Handle empty/degenerate cases
    if min_x > max_x {
        min_x = -1.0;
        max_x = 1.0;
    }
    if min_y > max_y {
        min_y = -1.0;
        max_y = 1.0;
    }

    (min_x, max_x, min_y, max_y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apvd_core::circle::Circle;
    use apvd_core::r2::R2;

    #[test]
    fn test_render_circles() {
        let shapes: Vec<Shape<D>> = vec![
            Shape::Circle(Circle {
                c: R2 { x: 0.0.into(), y: 0.0.into() },
                r: 1.0.into(),
            }),
            Shape::Circle(Circle {
                c: R2 { x: 1.0.into(), y: 0.0.into() },
                r: 1.0.into(),
            }),
        ];

        let svg = render_svg(&shapes, &RenderConfig::default());
        assert!(svg.contains("<svg"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("</svg>"));
    }
}
