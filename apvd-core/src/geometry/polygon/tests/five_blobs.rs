use super::super::*;
use crate::r2::R2;
use crate::shape::Shape;

fn five_shape_layout(n_sides: usize, dist: f64, rx: f64, ry: f64, dent: f64) -> Vec<Shape<f64>> {
    (0..5).map(|i| {
        let angle = std::f64::consts::FRAC_PI_2 + (2.0 * std::f64::consts::PI * i as f64) / 5.0;
        let cx = dist * angle.cos();
        let cy = dist * angle.sin();
        let rotation = angle;
        let cos_r = rotation.cos();
        let sin_r = rotation.sin();
        let vertices: Vec<R2<f64>> = (0..n_sides).map(|j| {
            let theta = 2.0 * std::f64::consts::PI * (j as f64) / (n_sides as f64);
            let cos_t = theta.cos();
            let sin_t = theta.sin();
            let r = 1.0 + dent * cos_t;
            let px = sin_t * rx * r;
            let py = cos_t * ry * r;
            R2 {
                x: cx + px * cos_r - py * sin_r,
                y: cy + px * sin_r + py * cos_r,
            }
        }).collect();
        Shape::Polygon(Polygon::new(vertices))
    }).collect()
}

fn count_regions(shapes: Vec<Shape<f64>>, label: &str) -> (usize, usize, usize) {
    use crate::scene::Scene;

    let scene: Scene<f64> = Scene::new(shapes).expect("Failed to create scene");

    let mut negative_regions = Vec::new();
    let mut zero_regions = Vec::new();
    let mut positive_regions = Vec::new();

    for mask in 1u32..32 {
        let key: String = (0..5).map(|i| {
            if mask & (1 << i) != 0 { char::from_digit(i, 10).unwrap() } else { '-' }
        }).collect();

        let area = scene.area(&key);
        let area_val: f64 = area.clone().unwrap_or(0.0);
        if area_val < -0.001 {
            negative_regions.push((key, area_val));
        } else if area_val.abs() < 0.001 {
            zero_regions.push(key);
        } else {
            positive_regions.push((key, area_val));
        }
    }

    eprintln!("\n=== {} ===", label);
    eprintln!("{} positive, {} zero, {} negative", positive_regions.len(), zero_regions.len(), negative_regions.len());
    if !zero_regions.is_empty() {
        eprintln!("  Zero: {}", zero_regions.join(", "));
    }
    if !negative_regions.is_empty() {
        eprintln!("  Negative: {}", negative_regions.iter().map(|(k, v)| format!("{}({:.4})", k, v)).collect::<Vec<_>>().join(", "));
    }

    (positive_regions.len(), zero_regions.len(), negative_regions.len())
}

#[test]
fn test_five_shape_layouts_region_count() {
    // 31/31 requires tight spacing so non-adjacent triples overlap.
    // Explore the boundary between 26/31 and 31/31.

    // Sweep dist from 0.15 to 0.30, with width=0.8, height=1.5
    for &dist in &[0.15, 0.18, 0.20, 0.22, 0.25, 0.28, 0.30] {
        let shapes = five_shape_layout(40, dist, 0.8, 1.5, 0.15);
        count_regions(shapes, &format!("d={:.2}, w=0.8, h=1.5, dent=0.15", dist));
    }

    // Sweep dist with width=0.7
    for &dist in &[0.15, 0.18, 0.20, 0.22, 0.25] {
        let shapes = five_shape_layout(40, dist, 0.7, 1.5, 0.15);
        count_regions(shapes, &format!("d={:.2}, w=0.7, h=1.5, dent=0.15", dist));
    }

    // Sweep width at dist=0.2
    for &width in &[0.5, 0.6, 0.7, 0.8, 0.9, 1.0] {
        let shapes = five_shape_layout(40, 0.2, width, 1.5, 0.15);
        count_regions(shapes, &format!("d=0.20, w={:.1}, h=1.5, dent=0.15", width));
    }

    // Try dist=0.2 with different dents
    for &dent in &[0.0, 0.05, 0.10, 0.15, 0.20, 0.30] {
        let shapes = five_shape_layout(40, 0.2, 0.7, 1.5, dent);
        count_regions(shapes, &format!("d=0.20, w=0.7, h=1.5, dent={:.2}", dent));
    }
}

#[test]
fn test_five_blobs_vertex_count_sweep() {
    // Find minimum vertex count that still achieves 31 regions
    // with the standard five blobs layout params (d=0.2, w=0.7, h=1.5, dent=0.15)
    for &n in &[8, 10, 12, 15, 20, 25, 30, 40] {
        let shapes = five_shape_layout(n, 0.2, 0.7, 1.5, 0.15);
        let (pos, zero, neg) = count_regions(shapes, &format!("n={}", n));
        eprintln!("  n={}: {} positive, {} zero, {} negative", n, pos, zero, neg);
    }
}

#[test]
fn test_five_blobs_dual_vs_f64() {
    // Compare Scene<f64> vs Scene<Dual> (via Step::new) to isolate
    // whether the WASM region count discrepancy is a Dual vs f64 issue.
    use crate::step::Step;
    use crate::to::To;

    let shapes_f64 = five_shape_layout(40, 0.2, 0.7, 1.5, 0.15);
    let (p_f64, z_f64, n_f64) = count_regions(shapes_f64.clone(), "f64 Scene");

    // Create InputSpec for Dual scene (all coords trainable, like WASM does)
    let input_specs: Vec<(Shape<f64>, Vec<bool>)> = shapes_f64.iter().map(|s| {
        let n_coords = match s {
            Shape::Polygon(p) => p.vertices.len() * 2,
            _ => unreachable!(),
        };
        (s.clone(), vec![true; n_coords])
    }).collect();

    // Build targets for all 31 regions
    let mut targets_map: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
    for mask in 1u32..32 {
        let key: String = (0..5).map(|i| {
            if mask & (1 << i) != 0 { char::from_digit(i, 10).unwrap() } else { '-' }
        }).collect();
        targets_map.insert(key, 1.0);
    }

    let step = Step::new(input_specs, targets_map.into());
    match &step {
        Ok(step) => {
            let mut p_dual = 0;
            let mut z_dual = 0;
            let mut n_dual = 0;
            let mut negative_list = Vec::new();
            let mut zero_list = Vec::new();
            for (key, err) in &step.errors {
                if key.contains('*') { continue; }
                let actual = err.actual_area.unwrap_or(0.0);
                if actual < -0.001 {
                    n_dual += 1;
                    negative_list.push(format!("{}({:.1})", key, actual));
                } else if actual.abs() < 0.001 {
                    z_dual += 1;
                    zero_list.push(key.clone());
                } else {
                    p_dual += 1;
                }
            }
            eprintln!("\n=== Dual Scene ===");
            eprintln!("{} positive, {} zero, {} negative", p_dual, z_dual, n_dual);
            if !zero_list.is_empty() {
                eprintln!("  Zero: {}", zero_list.join(", "));
            }
            if !negative_list.is_empty() {
                eprintln!("  Negative: {}", negative_list.join(", "));
            }

            // The key comparison: do f64 and Dual agree?
            eprintln!("\nf64: {} positive, {} zero, {} negative", p_f64, z_f64, n_f64);
            eprintln!("Dual: {} positive, {} zero, {} negative", p_dual, z_dual, n_dual);
        }
        Err(e) => {
            eprintln!("Step::new failed: {:?}", e);
        }
    }
}

#[test]
fn test_five_blobs_verify_areas() {
    // Verify that shape.area() matches sum of region areas (catches CW winding sign bug)
    use crate::scene::Scene;

    for &n in &[12, 15, 20, 40] {
        let shapes = five_shape_layout(n, 0.2, 0.7, 1.5, 0.15);
        let scene: Scene<f64> = Scene::new(shapes).expect("Failed to create scene");
        for component in &scene.components {
            component.verify_areas(0.01).unwrap_or_else(|e| {
                panic!("verify_areas failed for n={}: {}", n, e);
            });
        }
    }
}

/// Returns (num_positive_regions, min_area/total_area ratio, areas_vec) for a 5-shape layout.
fn region_stats(shapes: Vec<Shape<f64>>) -> (usize, f64, Vec<(String, f64)>) {
    use crate::scene::Scene;
    let scene: Scene<f64> = match Scene::new(shapes) {
        Ok(s) => s,
        Err(_) => return (0, 0.0, vec![]),
    };
    let mut areas = Vec::new();
    let mut min_positive = f64::INFINITY;
    let mut total_area = 0.0_f64;
    let mut n_positive = 0usize;
    for mask in 1u32..32 {
        let key: String = (0..5).map(|i| {
            if mask & (1 << i) != 0 { char::from_digit(i, 10).unwrap() } else { '-' }
        }).collect();
        let area_val: f64 = scene.area(&key).unwrap_or(0.0);
        areas.push((key, area_val));
        if area_val > 0.001 {
            n_positive += 1;
            total_area += area_val;
            if area_val < min_positive {
                min_positive = area_val;
            }
        }
    }
    if min_positive == f64::INFINITY { min_positive = 0.0; }
    let ratio = if total_area > 0.0 { min_positive / total_area } else { 0.0 };
    (n_positive, ratio, areas)
}

#[test]
fn test_five_blobs_optimize_min_region() {
    // Grid search over blob layout params to maximize the minimum region area.
    // Only configs achieving all 31 regions are considered.
    // Parameters: dist, width, height, dent (n_sides fixed at 15 and 12).

    #[derive(Clone)]
    struct Config {
        n: usize,
        dist: f64,
        width: f64,
        height: f64,
        dent: f64,
        n_regions: usize,
        min_area: f64,
    }

    let mut results: Vec<Config> = Vec::new();

    // Broad grid search (ratio = min_area / total_area is scale-invariant)
    for &n in &[12, 15] {
        for &dist in &[0.10, 0.15, 0.20, 0.25, 0.30] {
            for &width in &[0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.4, 1.6] {
                for &height in &[1.3, 1.5, 1.7, 1.9, 2.1, 2.5, 3.0] {
                    for &dent in &[0.0, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35] {
                        let shapes = five_shape_layout(n, dist, width, height, dent);
                        let (n_regions, min_area, _) = region_stats(shapes);
                        results.push(Config { n, dist, width, height, dent, n_regions, min_area });
                    }
                }
            }
        }
    }

    // Filter to 31-region configs, sort by ratio descending
    let mut full_configs: Vec<Config> = results.iter()
        .filter(|c| c.n_regions == 31)
        .cloned()
        .collect();
    full_configs.sort_by(|a, b| b.min_area.partial_cmp(&a.min_area).unwrap());

    eprintln!("\n=== Top 20 configs (31 regions, by min/total ratio) ===");
    for (i, c) in full_configs.iter().take(20).enumerate() {
        eprintln!(
            "  {:2}. n={:2} d={:.2} w={:.2} h={:.1} dent={:.2} → ratio={:.6}",
            i + 1, c.n, c.dist, c.width, c.height, c.dent, c.min_area,
        );
    }

    // Show best for each n separately
    for &n in &[12, 15] {
        if let Some(best) = full_configs.iter().find(|c| c.n == n) {
            eprintln!(
                "\nBest n={}: d={:.2} w={:.2} h={:.1} dent={:.2} → ratio={:.6}",
                n, best.dist, best.width, best.height, best.dent, best.min_area,
            );

            // Print all region areas for the best config
            let shapes = five_shape_layout(n, best.dist, best.width, best.height, best.dent);
            let (_, _, areas) = region_stats(shapes);
            let total: f64 = areas.iter().filter(|(_, a)| *a > 0.001).map(|(_, a)| a).sum();
            let mut sorted_areas: Vec<_> = areas.iter().filter(|(_, a)| *a > 0.001).collect();
            sorted_areas.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            eprintln!("  Regions (smallest first, total={:.4}):", total);
            for (key, area) in &sorted_areas {
                eprintln!("    {}: {:.6} ({:.4}%)", key, area, area / total * 100.0);
            }
        }
    }

    // Verify optimized defaults achieve 31 regions with good ratios
    let optimized = [
        (12, 0.10, 0.70, 1.3, 0.25),
        (15, 0.15, 1.40, 2.1, 0.15),
    ];
    for &(n, dist, width, height, dent) in &optimized {
        let shapes = five_shape_layout(n, dist, width, height, dent);
        let (n_regions, ratio, _) = region_stats(shapes);
        eprintln!("\nOptimized n={}: d={} w={} h={} dent={} → {} regions, ratio={:.6}",
            n, dist, width, height, dent, n_regions, ratio);
        assert_eq!(n_regions, 31, "Optimized n={} layout should have 31 regions", n);
        assert!(ratio > 0.006, "Optimized n={} layout ratio should be > 0.6%", n);
    }
}

#[test]
fn test_five_dodecagons_model_errors() {
    use crate::model::Model;
    use crate::to::To;

    // Create 5 elongated 12-gons in a pentagonal arrangement
    let dist = 0.5_f64;
    let rx = 0.45_f64;
    let ry = 1.3_f64;
    let n_sides = 12;

    let shapes: Vec<Shape<f64>> = (0..5).map(|i| {
        let angle = std::f64::consts::FRAC_PI_2 + (2.0 * std::f64::consts::PI * i as f64) / 5.0;
        let cx = dist * angle.cos();
        let cy = dist * angle.sin();
        let rotation = angle - std::f64::consts::FRAC_PI_2;
        let cos_r = rotation.cos();
        let sin_r = rotation.sin();
        let vertices: Vec<R2<f64>> = (0..n_sides).map(|j| {
            let theta = 2.0 * std::f64::consts::PI * (j as f64) / (n_sides as f64);
            let px = rx * theta.cos();
            let py = ry * theta.sin();
            R2 {
                x: cx + px * cos_r - py * sin_r,
                y: cy + px * sin_r + py * cos_r,
            }
        }).collect();
        Shape::Polygon(Polygon::new(vertices))
    }).collect();

    // FizzBuzzBazzQuxQuux exclusive targets (from sample-targets comment)
    let targets: [(&str, f64); 31] = [
        ("0----", 5280.),
        ("-1---", 2640.),
        ("--2--", 1320.),
        ("---3-", 1760.),
        ("----4",  880.),
        ("01---",  440.),
        ("0-2--",  220.),
        ("0--3-", 1056.),
        ("0---4",  528.),
        ("-12--",  264.),
        ("-1-3-",  132.),
        ("-1--4",  176.),
        ("--23-",   88.),
        ("--2-4",   44.),
        ("---34",   22.),
        ("012--",  480.),
        ("01-3-",  240.),
        ("01--4",  120.),
        ("0-23-",   60.),
        ("0-2-4",   80.),
        ("0--34",   40.),
        ("-123-",   20.),
        ("-12-4",   10.),
        ("-1-34",   48.),
        ("--234",   24.),
        ("0123-",   12.),
        ("012-4",    6.),
        ("01-34",    8.),
        ("0-234",    4.),
        ("-1234",    2.),
        ("01234",    1.),
    ];

    let inputs: Vec<(Shape<f64>, Vec<bool>)> = shapes
        .into_iter()
        .map(|s| {
            let ncoords = if let Shape::Polygon(ref p) = s { p.vertices.len() * 2 } else { 0 };
            (s, vec![true; ncoords])
        })
        .collect();

    let model = Model::new(inputs, targets.to()).expect("Failed to create model");
    let step = &model.steps[0];

    eprintln!("\n=== 5 Dodecagon Model Initial Errors ===");
    eprintln!("Total error: {:.4}", step.error.v());

    // Print errors for each target, sorted by absolute error
    let mut errors: Vec<_> = step.errors.iter()
        .map(|(key, err)| (key.clone(), err.actual_area, err.target_area, err.actual_frac, err.target_frac, err.error.v()))
        .collect();
    errors.sort_by(|a, b| b.5.abs().partial_cmp(&a.5.abs()).unwrap());

    for (key, actual_area, target_area, actual_frac, target_frac, error) in &errors {
        eprintln!("  {} actual_area={:>8.2?} target={:>6.0} actual_frac={:>8.4} target_frac={:>8.4} err={:>8.4}",
            key, actual_area, target_area, actual_frac, target_frac, error);
    }
}

/// Create a single blob shape: concave polygon at (cx,cy) rotated by `rotation` radians.
fn blob_shape(n_sides: usize, cx: f64, cy: f64, rx: f64, ry: f64, dent: f64, rotation: f64) -> Shape<f64> {
    let cos_r = rotation.cos();
    let sin_r = rotation.sin();
    let vertices: Vec<R2<f64>> = (0..n_sides).map(|j| {
        let theta = 2.0 * std::f64::consts::PI * (j as f64) / (n_sides as f64);
        let cos_t = theta.cos();
        let sin_t = theta.sin();
        let r = 1.0 + dent * cos_t;
        let px = sin_t * rx * r;
        let py = cos_t * ry * r;
        R2 {
            x: cx + px * cos_r - py * sin_r,
            y: cy + px * sin_r + py * cos_r,
        }
    }).collect();
    Shape::Polygon(Polygon::new(vertices))
}

#[test]
fn test_optimized_templates_not_self_intersecting() {
    // The radial optimizer output is deeply concave but NOT self-intersecting.
    // Region detection still fails on these shapes due to concavity-related bugs.

    // 12-gon template
    let template12: Vec<R2<f64>> = vec![
        R2 { x:  0.000000, y:  0.708513 },
        R2 { x:  0.135692, y:  0.235026 },
        R2 { x:  0.375308, y:  0.216684 },
        R2 { x:  0.070803, y:  0.000000 },
        R2 { x:  0.316522, y: -0.182744 },
        R2 { x:  0.300825, y: -0.521044 },
        R2 { x:  0.000000, y: -0.096645 },
        R2 { x: -0.275279, y: -0.476797 },
        R2 { x: -0.183135, y: -0.105733 },
        R2 { x: -0.056255, y:  0.000000 },
        R2 { x: -0.339773, y:  0.196168 },
        R2 { x: -0.144770, y:  0.250750 },
    ];
    let p12 = Polygon::new(template12);
    assert!(!p12.is_self_intersecting(), "12-gon template is deeply concave but not self-intersecting");

    // 15-gon template
    let template15: Vec<R2<f64>> = vec![
        R2 { x:  0.000000, y:  1.308383 },
        R2 { x:  0.548921, y:  1.232896 },
        R2 { x:  0.798618, y:  0.719079 },
        R2 { x:  0.253762, y:  0.082452 },
        R2 { x:  0.142250, y: -0.014951 },
        R2 { x:  0.781661, y: -0.451292 },
        R2 { x:  0.607416, y: -0.836036 },
        R2 { x:  0.099147, y: -0.466452 },
        R2 { x: -0.206830, y: -0.973058 },
        R2 { x: -0.754323, y: -1.038237 },
        R2 { x: -1.223385, y: -0.706322 },
        R2 { x: -0.202345, y: -0.021267 },
        R2 { x: -0.749854, y:  0.243642 },
        R2 { x: -1.170053, y:  1.053520 },
        R2 { x: -0.497034, y:  1.116356 },
    ];
    let p15 = Polygon::new(template15);
    assert!(!p15.is_self_intersecting(), "15-gon template is deeply concave but not self-intersecting");
}

/// Rotate a template polygon by `angle` radians.
fn rotate_template(template: &[R2<f64>], angle: f64) -> Shape<f64> {
    let cos = angle.cos();
    let sin = angle.sin();
    let vertices = template.iter().map(|v| R2 {
        x: v.x * cos - v.y * sin,
        y: v.x * sin + v.y * cos,
    }).collect();
    Shape::Polygon(Polygon::new(vertices))
}

#[test]
fn test_optimized_template_two_shapes_regions() {
    // Repro: 2 deeply concave shapes from the optimized 15-gon template.
    // The user reported missing `35` region and broken `3-` region.
    use crate::scene::Scene;

    let template: Vec<R2<f64>> = vec![
        R2 { x:  0.000000, y:  1.308383 },
        R2 { x:  0.548921, y:  1.232896 },
        R2 { x:  0.798618, y:  0.719079 },
        R2 { x:  0.253762, y:  0.082452 },
        R2 { x:  0.142250, y: -0.014951 },
        R2 { x:  0.781661, y: -0.451292 },
        R2 { x:  0.607416, y: -0.836036 },
        R2 { x:  0.099147, y: -0.466452 },
        R2 { x: -0.206830, y: -0.973058 },
        R2 { x: -0.754323, y: -1.038237 },
        R2 { x: -1.223385, y: -0.706322 },
        R2 { x: -0.202345, y: -0.021267 },
        R2 { x: -0.749854, y:  0.243642 },
        R2 { x: -1.170053, y:  1.053520 },
        R2 { x: -0.497034, y:  1.116356 },
    ];

    // Use shapes at positions 2 and 4 (i.e. shapes "3" and "5" from the 5-set layout)
    // to reproduce the user's {3,5} bug
    for (i, j) in [(0, 1), (0, 2), (2, 4)] {
        let angle_i = std::f64::consts::FRAC_PI_2 + (2.0 * std::f64::consts::PI * i as f64) / 5.0;
        let angle_j = std::f64::consts::FRAC_PI_2 + (2.0 * std::f64::consts::PI * j as f64) / 5.0;
        let shapes = vec![
            rotate_template(&template, angle_i),
            rotate_template(&template, angle_j),
        ];

        let scene = match Scene::new(shapes) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("shapes ({}, {}): Scene::new failed: {:?}", i, j, e);
                continue;
            }
        };

        let mut regions = Vec::new();
        for mask in 1u32..4 {
            let key: String = (0..2).map(|k| {
                if mask & (1 << k) != 0 { char::from_digit(k, 10).unwrap() } else { '-' }
            }).collect();
            let area = scene.area(&key).unwrap_or(0.0);
            regions.push((key, area));
        }

        let positives: Vec<_> = regions.iter().filter(|(_, a)| *a > 0.001).collect();
        eprintln!(
            "shapes ({}, {}): {} positive | {}",
            i, j, positives.len(),
            regions.iter().map(|(k, a)| format!("{}={:.4}", k, a)).collect::<Vec<_>>().join(", ")
        );

        // Print component details if not all 3 regions found
        if positives.len() != 3 {
            eprintln!("  BUG: Expected 3 regions");
            for (ci, component) in scene.components.iter().enumerate() {
                eprintln!("  Component {}: key={}, {} regions, {} edges, {} nodes",
                    ci, component.key, component.regions.len(),
                    component.edges.len(), component.nodes.len());
                for region in &component.regions {
                    eprintln!("    Region {}: area={:.6}, {} segments", region.key, region.total_area, region.segments.len());
                }
            }
        }
    }
}

#[test]
fn test_optimized_template_dual_vs_f64() {
    // Compare Scene<f64> (works) vs Scene<Dual> via Step::new
    // to see if the Dual pipeline produces different results.
    use crate::step::Step;
    use crate::scene::Scene;

    let template: Vec<R2<f64>> = vec![
        R2 { x:  0.000000, y:  1.308383 },
        R2 { x:  0.548921, y:  1.232896 },
        R2 { x:  0.798618, y:  0.719079 },
        R2 { x:  0.253762, y:  0.082452 },
        R2 { x:  0.142250, y: -0.014951 },
        R2 { x:  0.781661, y: -0.451292 },
        R2 { x:  0.607416, y: -0.836036 },
        R2 { x:  0.099147, y: -0.466452 },
        R2 { x: -0.206830, y: -0.973058 },
        R2 { x: -0.754323, y: -1.038237 },
        R2 { x: -1.223385, y: -0.706322 },
        R2 { x: -0.202345, y: -0.021267 },
        R2 { x: -0.749854, y:  0.243642 },
        R2 { x: -1.170053, y:  1.053520 },
        R2 { x: -0.497034, y:  1.116356 },
    ];

    // 2 shapes
    let angle0 = std::f64::consts::FRAC_PI_2 + (2.0 * std::f64::consts::PI * 2.0) / 5.0;
    let angle1 = std::f64::consts::FRAC_PI_2 + (2.0 * std::f64::consts::PI * 4.0) / 5.0;
    let shapes = vec![
        rotate_template(&template, angle0),
        rotate_template(&template, angle1),
    ];

    // f64 scene
    let scene_f64: Scene<f64> = Scene::new(shapes.clone()).expect("f64 scene");
    eprintln!("\n=== f64 Scene ===");
    for component in &scene_f64.components {
        eprintln!("Component {}: {} regions, {} edges", component.key, component.regions.len(), component.edges.len());
        for region in &component.regions {
            eprintln!("  Region {}: area={:.6}", region.key, region.total_area);
        }
    }

    // Dual scene via Step::new
    let input_specs: Vec<(Shape<f64>, Vec<bool>)> = shapes.iter().map(|s| {
        let n_coords = match s {
            Shape::Polygon(p) => p.vertices.len() * 2,
            _ => unreachable!(),
        };
        (s.clone(), vec![true; n_coords])
    }).collect();

    let mut targets_map: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
    for mask in 1u32..4 {
        let key: String = (0..2).map(|i| {
            if mask & (1 << i) != 0 { char::from_digit(i, 10).unwrap() } else { '-' }
        }).collect();
        targets_map.insert(key, 1.0);
    }

    let step = Step::new(input_specs, targets_map.into());
    match &step {
        Ok(step) => {
            eprintln!("\n=== Dual Scene ===");
            for (key, err) in &step.errors {
                if key.contains('*') { continue; }
                let actual = err.actual_area.unwrap_or(0.0);
                eprintln!("  {}: actual_area={:.6}", key, actual);
            }
        }
        Err(e) => {
            eprintln!("Step::new failed: {:?}", e);
        }
    }
}

#[test]
fn test_two_blobs_region_detection() {
    // Test region detection with 2 concave blob shapes at various concavity levels.
    // This isolates the specific bug where regions go missing for concave polygons.
    use crate::scene::Scene;

    let n = 15;
    let dist = 0.2;
    let rx = 0.7;
    let ry = 1.5;

    // Two shapes positioned like shapes 0 and 1 from the 5-blob layout
    for &dent in &[0.0, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40, 0.50] {
        let angle0 = std::f64::consts::FRAC_PI_2;
        let angle1 = std::f64::consts::FRAC_PI_2 + 2.0 * std::f64::consts::PI / 5.0;
        let shapes = vec![
            blob_shape(n, dist * angle0.cos(), dist * angle0.sin(), rx, ry, dent, angle0),
            blob_shape(n, dist * angle1.cos(), dist * angle1.sin(), rx, ry, dent, angle1),
        ];

        let scene = match Scene::new(shapes) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("dent={:.2}: Scene::new failed: {:?}", dent, e);
                continue;
            }
        };

        let mut regions = Vec::new();
        for mask in 1u32..4 {
            let key: String = (0..2).map(|i| {
                if mask & (1 << i) != 0 { char::from_digit(i, 10).unwrap() } else { '-' }
            }).collect();
            let area = scene.area(&key).unwrap_or(0.0);
            regions.push((key, area));
        }

        let positives: Vec<_> = regions.iter().filter(|(_, a)| *a > 0.001).collect();
        let negatives: Vec<_> = regions.iter().filter(|(_, a)| *a < -0.001).collect();
        let zeros: Vec<_> = regions.iter().filter(|(_, a)| a.abs() <= 0.001).collect();

        eprintln!(
            "dent={:.2}: {} positive, {} zero, {} negative | {}",
            dent, positives.len(), zeros.len(), negatives.len(),
            regions.iter().map(|(k, a)| format!("{}={:.4}", k, a)).collect::<Vec<_>>().join(", ")
        );

        if dent <= 0.40 {
            // For reasonable concavity, all 3 regions should exist (0-, 01, -1)
            if positives.len() != 3 {
                eprintln!("  BUG: Expected 3 positive regions for dent={:.2}, got {}", dent, positives.len());
                // Print component/region details for debugging
                for (ci, component) in scene.components.iter().enumerate() {
                    eprintln!("  Component {}: key={}, {} regions, {} edges, {} nodes",
                        ci, component.key, component.regions.len(),
                        component.edges.len(), component.nodes.len());
                    for region in &component.regions {
                        eprintln!("    Region {}: area={:.6}", region.key, region.total_area);
                    }
                }
            }
        }
    }
}

#[test]
fn test_optimized_template_five_shapes() {
    use crate::scene::Scene;

    let template: Vec<R2<f64>> = vec![
        R2 { x:  0.000000, y:  1.308383 },
        R2 { x:  0.548921, y:  1.232896 },
        R2 { x:  0.798618, y:  0.719079 },
        R2 { x:  0.253762, y:  0.082452 },
        R2 { x:  0.142250, y: -0.014951 },
        R2 { x:  0.781661, y: -0.451292 },
        R2 { x:  0.607416, y: -0.836036 },
        R2 { x:  0.099147, y: -0.466452 },
        R2 { x: -0.206830, y: -0.973058 },
        R2 { x: -0.754323, y: -1.038237 },
        R2 { x: -1.223385, y: -0.706322 },
        R2 { x: -0.202345, y: -0.021267 },
        R2 { x: -0.749854, y:  0.243642 },
        R2 { x: -1.170053, y:  1.053520 },
        R2 { x: -0.497034, y:  1.116356 },
    ];

    let shapes: Vec<Shape<f64>> = (0..5).map(|i| {
        let angle = std::f64::consts::FRAC_PI_2 + (2.0 * std::f64::consts::PI * i as f64) / 5.0;
        rotate_template(&template, angle)
    }).collect();

    let scene = match Scene::new(shapes) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Scene::new failed: {:?}", e);
            return;
        }
    };

    eprintln!("\n=== 5 optimized template shapes ===");
    for (ci, component) in scene.components.iter().enumerate() {
        eprintln!("Component {}: key={}, {} regions, {} edges, {} nodes",
            ci, component.key, component.regions.len(),
            component.edges.len(), component.nodes.len());
    }

    // Check all 31 region keys
    let mut positives = 0;
    let mut zeros = 0;
    let mut negatives = 0;
    for mask in 1u32..32 {
        let key: String = (0..5).map(|i| {
            if mask & (1 << i) != 0 { char::from_digit(i, 10).unwrap() } else { '-' }
        }).collect();
        let area = scene.area(&key).unwrap_or(0.0);
        if area > 0.001 {
            positives += 1;
        } else if area < -0.001 {
            negatives += 1;
            eprintln!("  NEGATIVE: {}={:.6}", key, area);
        } else {
            zeros += 1;
            eprintln!("  ZERO: {}={:.6}", key, area);
        }
    }
    eprintln!("{} positive, {} zero, {} negative", positives, zeros, negatives);
}

#[test]
fn test_12gon_opt_template_two_shapes_regions() {
    // Repro: the 12-gon opt template shapes 0,1 fail region detection in the browser.
    // Error: total_visits (42) != total_expected_visits (70)
    use crate::scene::Scene;

    let template12: Vec<R2<f64>> = vec![
        R2 { x:  0.000000, y:  0.708513 },
        R2 { x:  0.135692, y:  0.235026 },
        R2 { x:  0.375308, y:  0.216684 },
        R2 { x:  0.070803, y:  0.000000 },
        R2 { x:  0.316522, y: -0.182744 },
        R2 { x:  0.300825, y: -0.521044 },
        R2 { x:  0.000000, y: -0.096645 },
        R2 { x: -0.275279, y: -0.476797 },
        R2 { x: -0.183135, y: -0.105733 },
        R2 { x: -0.056255, y:  0.000000 },
        R2 { x: -0.339773, y:  0.196168 },
        R2 { x: -0.144770, y:  0.250750 },
    ];

    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let angle_i = std::f64::consts::FRAC_PI_2 + (2.0 * std::f64::consts::PI * i as f64) / 5.0;
        let angle_j = std::f64::consts::FRAC_PI_2 + (2.0 * std::f64::consts::PI * j as f64) / 5.0;
        let shapes = vec![
            rotate_template(&template12, angle_i),
            rotate_template(&template12, angle_j),
        ];

        let scene = match Scene::new(shapes) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("12-gon opt ({}, {}): Scene::new failed: {:?}", i, j, e);
                continue;
            }
        };

        let mut regions = Vec::new();
        for mask in 1u32..4 {
            let key: String = (0..2).map(|k| {
                if mask & (1 << k) != 0 { char::from_digit(k, 10).unwrap() } else { '-' }
            }).collect();
            let area = scene.area(&key).unwrap_or(0.0);
            regions.push((key, area));
        }

        let positives: Vec<_> = regions.iter().filter(|(_, a)| *a > 0.001).collect();
        if positives.len() != 3 {
            eprintln!(
                "12-gon opt ({}, {}): {} positive | {}",
                i, j, positives.len(),
                regions.iter().map(|(k, a)| format!("{}={:.4}", k, a)).collect::<Vec<_>>().join(", ")
            );
            for (ci, component) in scene.components.iter().enumerate() {
                eprintln!("  Component {}: key={}, {} regions, {} edges, {} nodes",
                    ci, component.key, component.regions.len(),
                    component.edges.len(), component.nodes.len());
                for region in &component.regions {
                    eprintln!("    Region {}: area={:.6}, {} segments", region.key, region.total_area, region.segments.len());
                }
            }
        }
        assert_eq!(positives.len(), 3, "12-gon opt ({}, {}): expected 3 positive regions", i, j);
    }
}
