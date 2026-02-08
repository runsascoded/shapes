//! Optimize a 5-shape layout by training a single template polygon.
//!
//! The template is parameterized as N radii at fixed uniform angles (polar
//! coordinates). This guarantees the polygon stays simple (non-self-intersecting)
//! since vertices maintain their angular ordering. The radii are replicated 5×
//! via 72° rotations, and the loss function drives all 31 region areas toward
//! equality. Gradients flow through Scene<Dual> back to the radii, and Adam
//! handles the updates.

use std::f64::consts::PI;

use apvd_core::dual::Dual;
use apvd_core::geometry::polygon::Polygon;
use apvd_core::optimization::adam::AdamState;
use apvd_core::r2::R2;
use apvd_core::scene::Scene;
use apvd_core::shape::Shape;

/// Result of a layout optimization run.
pub struct LayoutResult {
    /// Optimized template vertices (before rotation)
    pub template: Vec<R2<f64>>,
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

/// Wrap f64 radii as Dual numbers with one-hot gradient vectors.
fn dualize_radii(radii: &[f64]) -> Vec<Dual> {
    let n_params = radii.len();
    radii.iter().enumerate().map(|(i, &r)| {
        let mut d = vec![0.0; n_params];
        d[i] = 1.0;
        Dual::new(r, d)
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

/// Generate 5 polygon shapes from a template by rotating 72° apart.
fn replicate_template(template: &[R2<Dual>], n_params: usize) -> Vec<Shape<Dual>> {
    (0..5).map(|i| {
        let angle = PI / 2.0 + (2.0 * PI * i as f64) / 5.0;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let vertices: Vec<R2<Dual>> = template.iter().map(|v| {
            let cos_d = Dual::scalar(cos_a, n_params);
            let sin_d = Dual::scalar(sin_a, n_params);
            let x = v.x.clone() * &cos_d - v.y.clone() * &sin_d;
            let y = v.x.clone() * &sin_d + v.y.clone() * &cos_d;
            R2 { x, y }
        }).collect();
        Shape::Polygon(Polygon::new(vertices))
    }).collect()
}

/// Compute all 31 region areas from 5 shapes.
/// Returns (areas_vec, total_area) or None if Scene fails.
fn compute_regions(shapes: Vec<Shape<Dual>>, n_params: usize) -> Option<(Vec<(String, Dual)>, Dual)> {
    let scene = Scene::new(shapes).ok()?;
    let mut areas = Vec::new();
    let mut total = Dual::zero(n_params);
    for mask in 1u32..32 {
        let key: String = (0..5).map(|i| {
            if mask & (1 << i) != 0 { char::from_digit(i, 10).unwrap() } else { '-' }
        }).collect();
        let area = scene.area(&key).unwrap_or_else(|| Dual::zero(n_params));
        total = total + area.clone();
        areas.push((key, area));
    }
    Some((areas, total))
}

/// Run the layout optimization.
pub fn optimize_layout(
    n_vertices: usize,
    max_steps: usize,
    learning_rate: f64,
    init_width: f64,
    init_height: f64,
    init_dent: f64,
) -> Result<LayoutResult, String> {
    let n_params = n_vertices;
    let mut radii = initial_radii(n_vertices, init_width, init_height, init_dent);
    let mut adam = AdamState::new(n_params);
    let mut loss_history = Vec::with_capacity(max_steps);
    let mut best_loss = f64::INFINITY;
    let mut best_radii = radii.clone();
    let mut best_step = 0;

    for step in 0..max_steps {
        let dual_radii = dualize_radii(&radii);
        let vertices = dual_radii_to_vertices(&dual_radii, n_params);
        let shapes = replicate_template(&vertices, n_params);

        let (areas, total) = compute_regions(shapes, n_params)
            .ok_or_else(|| format!("Scene creation failed at step {}", step))?;

        // Count positive regions
        let n_positive = areas.iter().filter(|(_, a)| a.v() > 1e-6).count();

        // Loss = sum of (area_i - mean)^2 (variance * 31)
        let mean = total.clone() / Dual::scalar(31.0, n_params);
        let mut loss = Dual::zero(n_params);
        for (_, area) in &areas {
            let diff = area - mean.clone();
            loss = loss + &diff * &diff;
        }

        // Add penalty for missing regions (area <= 0)
        for (_, area) in &areas {
            if area.v() < 1e-6 {
                let penalty = Dual::scalar(10.0, n_params) - area.clone();
                loss = loss + &penalty * &penalty;
            }
        }

        let loss_val = loss.v();
        loss_history.push(loss_val);

        if loss_val < best_loss && n_positive == 31 {
            best_loss = loss_val;
            best_radii = radii.clone();
            best_step = step;
        }

        // Progress
        if step % 100 == 0 || step == max_steps - 1 {
            let min_area = areas.iter().map(|(_, a)| a.v()).filter(|a| *a > 1e-6).fold(f64::INFINITY, f64::min);
            let total_val = total.v();
            let ratio = if total_val > 0.0 && min_area < f64::INFINITY { min_area / total_val } else { 0.0 };
            eprintln!(
                "  step {:5}: loss={:.6e}  regions={}/31  min/total={:.4}%  (best: step {} loss={:.6e})",
                step, loss_val, n_positive, ratio * 100.0, best_step, best_loss,
            );
        }

        // Gradient descent step
        let grad = loss.d();
        let updates = adam.step(&grad, learning_rate);
        for (i, r) in radii.iter_mut().enumerate() {
            *r -= updates[i];
            // Keep radii positive (avoid degenerate/inverted polygons)
            if *r < 0.01 {
                *r = 0.01;
            }
        }
    }

    // Compute final stats from best radii
    let best_template = radii_to_vertices(&best_radii, n_vertices);
    let dual_radii = dualize_radii(&best_radii);
    let vertices = dual_radii_to_vertices(&dual_radii, n_params);
    let shapes = replicate_template(&vertices, n_params);
    let (areas, total) = compute_regions(shapes, n_params)
        .ok_or("Scene failed for best template")?;

    let total_val = total.v();
    let mut region_areas: Vec<(String, f64)> = areas.iter().map(|(k, a)| (k.clone(), a.v())).collect();
    region_areas.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let min_positive = region_areas.iter().filter(|(_, a)| *a > 1e-6).map(|(_, a)| *a).next().unwrap_or(0.0);
    let min_ratio = if total_val > 0.0 { min_positive / total_val } else { 0.0 };

    Ok(LayoutResult {
        template: best_template,
        loss_history,
        final_loss: best_loss,
        region_areas,
        min_ratio,
    })
}
