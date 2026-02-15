//! Optimize an N-shape layout by training a single template polygon.
//!
//! The template is parameterized as N radii at fixed uniform angles (polar
//! coordinates) plus a center offset and inter-shape rotation angle. This
//! guarantees the polygon stays simple (non-self-intersecting) since vertices
//! maintain their angular ordering. The radii are replicated N× via rotation,
//! each translated by the offset along its rotation direction. The loss
//! function drives all (2^N - 1) region areas toward equality. Gradients flow
//! through Scene<Dual> back to the radii, offset, and angle_step, and Adam
//! handles the updates.

use std::f64::consts::PI;

use apvd_core::dual::Dual;
use apvd_core::geometry::polygon::Polygon;
use apvd_core::optimization::adam::AdamState;
use apvd_core::r2::R2;
use apvd_core::scene::Scene;
use apvd_core::shape::Shape;

/// Result of a layout optimization run.
#[allow(dead_code)]
pub struct LayoutResult {
    /// Number of shapes
    pub num_shapes: usize,
    /// Optimized template vertices (before rotation/offset)
    pub template: Vec<R2<f64>>,
    /// Optimized center offset (each shape translated by this distance from origin)
    pub offset: f64,
    /// Optimized inter-shape rotation angle (radians)
    pub angle_step: f64,
    /// Loss at each step
    pub loss_history: Vec<f64>,
    /// Final loss (variance of region areas)
    pub final_loss: f64,
    /// Final region areas (sorted by key)
    pub region_areas: Vec<(String, f64)>,
    /// Min/total ratio
    pub min_ratio: f64,
}

/// Fixed angles for N vertices, uniformly spaced.
fn angles(n: usize) -> Vec<f64> {
    (0..n).map(|j| 2.0 * PI * (j as f64) / (n as f64)).collect()
}

/// Compute initial radii from a cardioid-blob template.
fn initial_radii(n_vertices: usize, width: f64, height: f64, dent: f64) -> Vec<f64> {
    let thetas = angles(n_vertices);
    thetas.iter().map(|&theta| {
        let cos_t = theta.cos();
        let sin_t = theta.sin();
        let r = 1.0 + dent * cos_t;
        let x = sin_t * width * r;
        let y = cos_t * height * r;
        (x * x + y * y).sqrt()
    }).collect()
}

/// Convert f64 radii to Cartesian vertices (for output).
fn radii_to_vertices(radii: &[f64], n_vertices: usize) -> Vec<R2<f64>> {
    let thetas = angles(n_vertices);
    radii.iter().zip(thetas.iter()).map(|(&r, &theta)| {
        R2 {
            x: r * theta.sin(),
            y: r * theta.cos(),
        }
    }).collect()
}

/// Convert Dual radii at fixed angles to Dual Cartesian vertices.
fn dual_radii_to_vertices(radii: &[Dual], n_params: usize) -> Vec<R2<Dual>> {
    let n = radii.len();
    let thetas = angles(n);
    radii.iter().zip(thetas.iter()).map(|(r, &theta)| {
        R2 {
            x: r.clone() * Dual::scalar(theta.sin(), n_params),
            y: r.clone() * Dual::scalar(theta.cos(), n_params),
        }
    }).collect()
}

/// Generate N polygon shapes from a template, each rotated by `i * angle_step`
/// from `start_angle` and translated by `offset` along its rotation direction.
/// Both `offset` and `angle_step` are Dual for gradient flow.
fn replicate_template(
    template: &[R2<Dual>],
    n_params: usize,
    num_shapes: usize,
    offset: &Dual,
    angle_step: &Dual,
) -> Vec<Shape<Dual>> {
    let start = Dual::scalar(PI / 2.0, n_params);
    (0..num_shapes).map(|i| {
        let angle = start.clone() + angle_step.clone() * Dual::scalar(i as f64, n_params);
        let cos_a = angle.clone().cos();
        let sin_a = angle.sin();
        // Translate each shape's center along its rotation direction
        let ox = offset.clone() * cos_a.clone();
        let oy = offset.clone() * sin_a.clone();
        let vertices: Vec<R2<Dual>> = template.iter().map(|v| {
            let x = v.x.clone() * cos_a.clone() - v.y.clone() * sin_a.clone() + ox.clone();
            let y = v.x.clone() * sin_a.clone() + v.y.clone() * cos_a.clone() + oy.clone();
            R2 { x, y }
        }).collect();
        Shape::Polygon(Polygon::new(vertices))
    }).collect()
}

struct RegionInfo {
    /// Summed area per key (2^N - 1 entries)
    areas: Vec<(String, Dual)>,
    /// Total area across all regions
    total: Dual,
    /// Fragment areas: for each key with >1 geometric region,
    /// the areas of all but the largest (the ones we want to eliminate)
    fragment_areas: Vec<Dual>,
}

/// Compute all (2^N - 1) region areas from N shapes, plus fragmentation info.
fn compute_regions(shapes: Vec<Shape<Dual>>, n_params: usize, num_shapes: usize) -> Option<RegionInfo> {
    let scene = Scene::new(shapes).ok()?;

    // Collect actual geometric regions grouped by key
    let mut regions_by_key: std::collections::BTreeMap<String, Vec<Dual>> = std::collections::BTreeMap::new();
    for component in &scene.components {
        for region in &component.regions {
            regions_by_key.entry(region.key.clone()).or_default().push(region.area());
        }
    }

    // Build summed areas for all (2^N - 1) keys, and collect fragment penalties
    let mut areas = Vec::new();
    let mut total = Dual::zero(n_params);
    let mut fragment_areas = Vec::new();

    for mask in 1u32..(1u32 << num_shapes) {
        let key: String = (0..num_shapes).map(|i| {
            if mask & (1 << i) != 0 { char::from_digit(i as u32, 10).unwrap() } else { '-' }
        }).collect();
        let area = scene.area(&key).unwrap_or_else(|| Dual::zero(n_params));
        total = total + area.clone();
        areas.push((key.clone(), area));

        // Check for fragmentation: if this key has multiple geometric regions,
        // penalize all but the largest
        if let Some(mut geo_areas) = regions_by_key.remove(&key) {
            if geo_areas.len() > 1 {
                // Sort descending by value, keep all but the largest as fragments
                geo_areas.sort_by(|a, b| b.v().partial_cmp(&a.v()).unwrap());
                for frag in geo_areas.into_iter().skip(1) {
                    fragment_areas.push(frag);
                }
            }
        }
    }

    Some(RegionInfo { areas, total, fragment_areas })
}

/// Run the layout optimization.
///
/// `init_offset`: initial center offset (each shape translated this far from
/// origin along its rotation direction). If `None`, auto-computed from template
/// parameters: for N>=5 the cardioid asymmetry provides sufficient implicit
/// offset, for N<5 a larger explicit offset is needed.
///
/// `init_angle_step`: inter-shape rotation angle in radians. If `None`,
/// auto-computed: `π/3` (60°) for N<5 (avoids 180° opposition that kills
/// opposite-pair regions), `2π / N` for N>=5.
pub fn optimize_layout(
    num_shapes: usize,
    n_vertices: usize,
    max_steps: usize,
    learning_rate: f64,
    init_width: f64,
    init_height: f64,
    init_dent: f64,
    init_offset: Option<f64>,
    init_angle_step: Option<f64>,
    fix_angle: bool,
) -> Result<LayoutResult, String> {
    let num_regions = (1usize << num_shapes) - 1;
    // Parameters: N radii + offset + angle_step
    let n_params = n_vertices + 2;
    let mut radii = initial_radii(n_vertices, init_width, init_height, init_dent);
    let mut offset = init_offset.unwrap_or_else(|| {
        if num_shapes >= 5 {
            0.0
        } else {
            init_width * 0.4
        }
    });
    let mut angle_step = init_angle_step.unwrap_or_else(|| {
        if num_shapes >= 5 {
            2.0 * PI / num_shapes as f64
        } else {
            // For even N, use 60° (π/3) to avoid the 180° opposition that
            // kills opposite-pair regions. Empirically, 60° finds the best
            // basin for N=4 (min/total ~5.3%, all 15 regions present).
            PI / 3.0
        }
    });
    eprintln!("  Center offset: {:.4}, angle_step: {:.4}° ({:.4} rad)", offset, angle_step.to_degrees(), angle_step);
    let mut adam = AdamState::new(n_params);
    let mut lr = learning_rate;
    let mut loss_history = Vec::with_capacity(max_steps);
    let mut best_loss = f64::INFINITY;
    let mut best_radii = radii.clone();
    let mut best_offset = offset;
    let mut best_angle_step = angle_step;
    let mut best_step = 0;
    let mut steps_since_best = 0usize;
    let mut restarts = 0usize;

    for step in 0..max_steps {
        // Restart from best if stuck with missing regions for too long
        if steps_since_best > 200 && best_loss < f64::INFINITY {
            restarts += 1;
            if restarts > 5 {
                eprintln!("  step {:5}: max restarts reached, stopping", step);
                break;
            }
            lr *= 0.5;
            radii = best_radii.clone();
            offset = best_offset;
            angle_step = best_angle_step;
            adam = AdamState::new(n_params);
            steps_since_best = 0;
            eprintln!("  step {:5}: restart #{} from best (step {}), lr={:.6}", step, restarts, best_step, lr);
            continue;
        }

        // Dualize radii (params 0..n_vertices)
        let dual_radii: Vec<Dual> = radii.iter().enumerate().map(|(i, &r)| {
            let mut d = vec![0.0; n_params];
            d[i] = 1.0;
            Dual::new(r, d)
        }).collect();
        // Dualize offset (param n_vertices)
        let offset_dual = {
            let mut d = vec![0.0; n_params];
            d[n_vertices] = 1.0;
            Dual::new(offset, d)
        };
        // Dualize angle_step (param n_vertices + 1)
        let angle_step_dual = {
            let mut d = vec![0.0; n_params];
            d[n_vertices + 1] = 1.0;
            Dual::new(angle_step, d)
        };

        let vertices = dual_radii_to_vertices(&dual_radii, n_params);
        let shapes = replicate_template(&vertices, n_params, num_shapes, &offset_dual, &angle_step_dual);

        let info = compute_regions(shapes, n_params, num_shapes)
            .ok_or_else(|| format!("Scene creation failed at step {}", step))?;

        // Count positive regions
        let n_positive = info.areas.iter().filter(|(_, a)| a.v() > 1e-6).count();
        let n_fragments = info.fragment_areas.len();

        // Loss: negative mean log-fraction (KL divergence from uniform).
        // Minimized when all fractions = 1/num_regions (uniform distribution).
        // Gradient ∝ -1/frac_i, giving strong signal for underrepresented
        // regions. This avoids the squared-error bias where many large regions
        // dominate gradient for few small regions.
        let mut loss = Dual::zero(n_params);
        let inv_n = Dual::scalar(1.0 / num_regions as f64, n_params);
        for (_, area) in &info.areas {
            let frac = area.clone() / &info.total;
            if frac.v() > 1e-12 {
                loss = loss - inv_n.clone() * frac.ln();
            } else {
                // Region effectively missing: large penalty
                loss = loss + Dual::scalar(10.0, n_params);
            }
        }

        // Fragmentation penalty: gentle L1 penalty on fragment fraction.
        // Keep this mild to avoid driving the optimizer away from valid layouts.
        if !info.fragment_areas.is_empty() {
            let frag_weight = Dual::scalar(1.0, n_params);
            for frag_area in &info.fragment_areas {
                let frag_frac = frag_area.clone() / &info.total;
                loss = loss + frag_weight.clone() * frag_frac;
            }
        }

        let loss_val = loss.v();
        loss_history.push(loss_val);

        if n_positive == num_regions && loss_val < best_loss {
            best_loss = loss_val;
            best_radii = radii.clone();
            best_offset = offset;
            best_angle_step = angle_step;
            best_step = step;
            steps_since_best = 0;
        } else {
            steps_since_best += 1;
        }

        // Progress
        if step % 100 == 0 || step == max_steps - 1 {
            let min_area = info.areas.iter().map(|(_, a)| a.v()).filter(|a| *a > 1e-6).fold(f64::INFINITY, f64::min);
            let total_val = info.total.v();
            let ratio = if total_val > 0.0 && min_area < f64::INFINITY { min_area / total_val } else { 0.0 };
            let frag_str = if n_fragments > 0 { format!("  frags={}", n_fragments) } else { String::new() };
            eprintln!(
                "  step {:5}: loss={:.6e}  regions={}/{}{}  min/total={:.4}%  offset={:.4}  angle={:.2}°  lr={:.6}  (best: step {} loss={:.6e})",
                step, loss_val, n_positive, num_regions, frag_str, ratio * 100.0, offset, angle_step.to_degrees(), lr, best_step, best_loss,
            );
        }

        // Gradient descent step
        let grad = loss.d();
        let updates = adam.step(&grad, lr);
        for (i, r) in radii.iter_mut().enumerate() {
            *r -= updates[i];
            if *r < 0.01 {
                *r = 0.01;
            }
        }
        offset -= updates[n_vertices];
        if offset < 0.0 {
            offset = 0.0;
        }
        if !fix_angle {
            angle_step -= updates[n_vertices + 1];
            // Clamp angle_step to reasonable range: [30°, 150°]
            angle_step = angle_step.clamp(PI / 6.0, 5.0 * PI / 6.0);
        }
    }

    // Compute final stats from best params
    let best_template = radii_to_vertices(&best_radii, n_vertices);
    let dual_radii: Vec<Dual> = best_radii.iter().enumerate().map(|(i, &r)| {
        let mut d = vec![0.0; n_params];
        d[i] = 1.0;
        Dual::new(r, d)
    }).collect();
    let offset_dual = {
        let mut d = vec![0.0; n_params];
        d[n_vertices] = 1.0;
        Dual::new(best_offset, d)
    };
    let angle_step_dual = {
        let mut d = vec![0.0; n_params];
        d[n_vertices + 1] = 1.0;
        Dual::new(best_angle_step, d)
    };
    let vertices = dual_radii_to_vertices(&dual_radii, n_params);
    let shapes = replicate_template(&vertices, n_params, num_shapes, &offset_dual, &angle_step_dual);
    let info = compute_regions(shapes, n_params, num_shapes)
        .ok_or("Scene failed for best template")?;

    let total_val = info.total.v();
    let mut region_areas: Vec<(String, f64)> = info.areas.iter().map(|(k, a)| (k.clone(), a.v())).collect();
    region_areas.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let min_positive = region_areas.iter().filter(|(_, a)| *a > 1e-6).map(|(_, a)| *a).next().unwrap_or(0.0);
    let min_ratio = if total_val > 0.0 { min_positive / total_val } else { 0.0 };

    if !info.fragment_areas.is_empty() {
        eprintln!("WARNING: Best template has {} fragments", info.fragment_areas.len());
    }

    Ok(LayoutResult {
        num_shapes,
        template: best_template,
        offset: best_offset,
        angle_step: best_angle_step,
        loss_history,
        final_loss: best_loss,
        region_areas,
        min_ratio,
    })
}
