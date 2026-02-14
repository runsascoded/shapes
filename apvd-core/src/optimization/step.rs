use std::collections::BTreeMap;
use std::fmt::Display;

use log::{debug, warn};
use serde::{Deserialize, Serialize};
use tsify::{declare, Tsify};

use crate::math::recip::Recip;
use crate::shape::{Shape, Shapes, InputSpec};
use crate::{distance::Distance, error::SceneError, scene::Scene, math::is_zero::IsZero, r2::R2, targets::Targets, regions};
use crate::dual::{Dual, D};
use super::adam::AdamState;

#[declare]
pub type Errors = BTreeMap<String, Error>;

/// Convergence threshold - below this error, consider the optimization converged.
pub const CONVERGENCE_THRESHOLD: f64 = 1e-10;

/// Per-penalty weights. Weight of 0 disables the penalty entirely.
#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct PenaltyConfig {
    pub disjoint: f64,
    pub contained: f64,
    pub fragmentation: f64,
    pub self_intersection: f64,
    pub regularity: f64,
    /// Per-shape isoperimetric ratio penalty
    pub shape_perimeter_area: f64,
    /// Per-DR isoperimetric ratio penalty
    pub region_perimeter_area: f64,
}

impl Default for PenaltyConfig {
    fn default() -> Self {
        PenaltyConfig {
            disjoint: 1.0,
            contained: 1.0,
            fragmentation: 1.0,
            self_intersection: 1.0,
            regularity: 0.0,  // disabled by default; P:A subsumes it
            shape_perimeter_area: 1.0,
            region_perimeter_area: 0.1,  // lower than shape_perimeter_area since regions can't independently reshape
        }
    }
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Step {
    pub shapes: Vec<Shape<D>>,
    pub components: Vec<regions::Component>,
    pub targets: Targets<f64>,
    pub total_area: Dual,
    pub errors: Errors,
    pub error: Dual,
    /// True if error is below convergence threshold (1e-10).
    /// Frontend should stop iterating when this is true.
    pub converged: bool,
    /// Breakdown of penalty terms (values only, gradients are folded into `error`)
    pub penalties: Penalties,
    /// Penalty weights used for this step
    pub penalty_config: PenaltyConfig,
    /// Per-shape gradient anchor points for visualization
    pub gradient_anchors: Vec<Vec<GradientAnchor>>,
}

/// A point on a shape where a gradient arrow should be drawn.
#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct GradientAnchor {
    /// Position of the anchor point (where to draw arrow base)
    pub position: R2<f64>,
    /// Error gradient projected onto this anchor's degrees of freedom.
    /// Frontend draws arrow in *negative* gradient direction (descent).
    pub gradient: R2<f64>,
    /// Human-readable label: "center", "0°", "90°", "v0", "v1", etc.
    pub label: String,
}

/// Classification of a per-region error.
#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ErrorKind {
    /// Region exists and has target > 0, but actual != target
    AreaMismatch {
        /// Positive = too large, negative = too small
        signed_error: f64,
    },
    /// Region should exist (target > 0) but doesn't (actual = 0 or None)
    MissingRegion {
        target_frac: f64,
    },
    /// Region exists (actual > 0) but shouldn't (target = 0)
    ExtraRegion {
        actual_frac: f64,
    },
}

/// Per-region penalty breakdown.
#[derive(Clone, Debug, Default, Tsify, Serialize, Deserialize)]
pub struct RegionPenalties {
    /// Total area of non-largest fragments (normalized by total_area)
    pub fragmentation: f64,
    /// Number of geometric components (1 = no fragmentation)
    pub fragment_count: usize,
    /// Penalty for shapes that should overlap here but are completely disjoint
    pub disjoint: f64,
    /// Penalty for shapes where parents exist but this intersection doesn't
    pub contained: f64,
    /// Isoperimetric ratio P²/(4πA) - 1 for this DR's boundary
    pub perimeter_area: f64,
}

#[declare]
pub type RegionPenaltiesMap = BTreeMap<String, RegionPenalties>;

/// Per-shape penalty breakdown (polygon-specific; all zeros for non-polygons).
#[derive(Clone, Debug, Default, Tsify, Serialize, Deserialize)]
pub struct ShapePenalties {
    /// Self-intersection penalty for this shape
    pub self_intersection: f64,
    /// Regularity penalty (edge variance + convexity) for this shape
    pub regularity: f64,
    /// Isoperimetric ratio penalty for this shape
    pub perimeter_area: f64,
}

/// Breakdown of penalty terms applied during optimization.
///
/// These values represent the scalar magnitudes of each penalty.
/// Gradients are folded into the `Step::error` dual for optimization,
/// but the values here are the raw (pre-zeroing) penalty magnitudes.
#[derive(Clone, Debug, Default, Tsify, Serialize, Deserialize)]
pub struct Penalties {
    /// Penalty for shapes that should overlap but are completely disjoint
    pub disjoint: f64,
    /// Penalty for shapes where parent regions exist but target intersection doesn't
    pub contained: f64,
    /// Polygon self-intersection penalty
    pub self_intersection: f64,
    /// Polygon regularity penalty (edge variance + convexity)
    pub regularity: f64,
    /// Fragmentation penalty: area of non-largest components per region key
    pub fragmentation: f64,
    /// Per-shape isoperimetric ratio penalty: P²/(4πA) - 1
    pub perimeter_area: f64,
    /// Per-DR isoperimetric ratio penalty (sum across all DRs)
    pub region_perimeter_area: f64,
    /// Per-region penalty breakdown
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub region_penalties: RegionPenaltiesMap,
    /// Per-shape penalty breakdown (one entry per shape)
    pub per_shape: Vec<ShapePenalties>,
}

impl Penalties {
    pub fn total(&self) -> f64 {
        self.disjoint + self.contained + self.self_intersection + self.regularity + self.fragmentation + self.perimeter_area + self.region_perimeter_area
    }
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Error {
    pub key: String,
    pub actual_area: Option<f64>,
    pub actual_frac: f64,
    pub target_area: f64,
    pub target_frac: f64,
    pub error: Dual,
    /// Classification of this error
    pub kind: ErrorKind,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "{}: err {:.3}, target {:.3} ({:.3}), actual {} → {:.3}",
            self.key, self.error.v(),
            self.target_area, self.target_frac,
            self.actual_area.map(|a| format!("{:.3}", a)).unwrap_or_else(|| "-".to_string()),
            self.actual_frac,
        )
    }
}

/// Dot product of two f64 slices.
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Compute gradient anchors for a shape, projected onto the error gradient.
fn shape_gradient_anchors(shape: &Shape<D>, error_d: &[f64]) -> Vec<GradientAnchor> {
    let anchor = |label: &str, pos: R2<Dual>| -> GradientAnchor {
        GradientAnchor {
            position: R2 { x: pos.x.v(), y: pos.y.v() },
            gradient: R2 { x: dot(&pos.x.d(), error_d), y: dot(&pos.y.d(), error_d) },
            label: label.to_string(),
        }
    };
    match shape {
        Shape::Circle(c) => vec![
            anchor("center", c.c.clone()),
            anchor("0°", R2 { x: c.c.x.clone() + c.r.clone(), y: c.c.y.clone() }),
        ],
        Shape::XYRR(e) => vec![
            anchor("center", e.c.clone()),
            anchor("0°", R2 { x: e.c.x.clone() + e.r.x.clone(), y: e.c.y.clone() }),
            anchor("90°", R2 { x: e.c.x.clone(), y: e.c.y.clone() + e.r.y.clone() }),
        ],
        Shape::XYRRT(e) => {
            let cos_t = e.t.clone().cos();
            let sin_t = e.t.clone().sin();
            vec![
                anchor("center", e.c.clone()),
                anchor("0°", R2 {
                    x: e.c.x.clone() + e.r.x.clone() * cos_t.clone(),
                    y: e.c.y.clone() + e.r.x.clone() * sin_t.clone(),
                }),
                anchor("90°", R2 {
                    x: e.c.x.clone() - e.r.y.clone() * sin_t,
                    y: e.c.y.clone() + e.r.y.clone() * cos_t,
                }),
            ]
        }
        Shape::Polygon(p) => {
            p.vertices.iter().enumerate().map(|(i, v)| {
                anchor(&format!("v{i}"), v.clone())
            }).collect()
        }
    }
}

impl Step {
    pub fn new(input_specs: Vec<InputSpec>, targets: Targets<f64>) -> Result<Step, SceneError> {
        let shapes = Shapes::from_vec(&input_specs);
        Step::nxt(shapes, targets, PenaltyConfig::default())
    }
    pub fn nxt(shapes: Vec<Shape<D>>, targets: Targets<f64>, penalty_config: PenaltyConfig) -> Result<Step, SceneError> {
        let scene = Scene::new(shapes)?;
        let sets = &scene.sets;
        let all_key = "*".repeat(scene.len());
        let total_area = scene.area(&all_key).unwrap_or_else(|| scene.zero());
        debug!("scene: {} components, total_area {}, component sizes {}", scene.components.len(), total_area, scene.components.iter().map(|c| c.sets.len().to_string()).collect::<Vec<_>>().join(", "));
        for component in &scene.components {
            debug!("  {} regions", component.regions.len());
            for region in &component.regions {
                debug!("    {}: {} segments, area {}", region.key, region.segments.len(), region.area());
            }
        }
        let errors = Self::compute_errors(&scene, &targets, &total_area);
        let disjoint_targets = targets.disjoints();
        let mut error = scene.zero();
        for key in disjoint_targets.keys() {
            let e = errors.get(key).unwrap();
            let err = e.error.abs();
            debug!("  {}: error {}, {}", key, e, err);
            error += err;
        }
        // let mut error: D = disjoint_targets.iter().map(|(key, _)| errors.get(key).unwrap().error.abs()).sum();
        debug!("step error {:?}", error);
        // Optional/Alternate loss function based on per-region squared errors, weights errors by region size:
        // let error = errors.values().into_iter().map(|e| e.error.clone() * &e.error).sum::<D>().sqrt();
        let components: Vec<regions::Component> = scene.components.iter().map(regions::Component::new).collect();
        debug!("{} components, num sets {}", components.len(), components.iter().map(|c| c.sets.len().to_string()).collect::<Vec<_>>().join(", "));

        // Include penalties for erroneously-disjoint shapes
        // let mut disjoint_penalties = Vec::<DisjointPenalty>::new();
        let mut total_disjoint_penalty = scene.zero();
        let mut total_contained_penalty = scene.zero();

        debug!("all targets: {}", targets.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<String>>().join(", "));
        let missing_regions: BTreeMap<String, f64> = disjoint_targets.into_iter().filter(|(key, target)| {
            let err = errors.get(key).unwrap_or_else(|| panic!("No key {} among error keys {}", key, errors.keys().cloned().collect::<Vec<String>>().join(", ")));
            let region_should_exist = target > &0.;
            let region_exists = err.actual_area.filter(|a| !a.is_zero()).is_some();
            region_should_exist && !region_exists
        }).collect();

        let mut total_missing_disjoint = 0.;
        let mut total_missing_contained = 0.;
        let mut per_key_disjoint_raw: BTreeMap<String, f64> = BTreeMap::new();
        let mut per_key_contained_raw: BTreeMap<String, f64> = BTreeMap::new();
        for (key, target) in missing_regions.iter() {
            let set_idxs: Vec<usize> = key.chars().enumerate().filter(|(_, c)| *c != '*' && *c != '-').map(|(idx, _)| idx).collect();
            let n = set_idxs.len();
            let nf = n as f64;
            let centroid: R2<Dual> = set_idxs.iter().map(|idx| sets[*idx].borrow().shape.center()).sum::<R2<Dual>>();
            let centroid = R2 { x: centroid.x / nf, y: centroid.y / nf };
            let parents_key = key.replace('-', "*");
            let parent_regions_exist = errors.get(&parents_key).unwrap().actual_area.filter(|a| !a.is_zero()).is_some();
            debug!("missing region {:?}, centroid {:?}, parents {} ({})", set_idxs, centroid, parents_key, parent_regions_exist);
            if parent_regions_exist {
                let mut parents = Vec::<usize>::new();
                for (idx, ch) in parents_key.char_indices() {
                    if ch == '*' {
                        let parent_key = format!("{}{}{}", &key[..idx], Targets::<f64>::idx(idx), &key[idx+1..]);
                        let parent_region_exists = errors.get(&parent_key).unwrap().actual_area.filter(|a| !a.is_zero()).is_some();
                        if parent_region_exists {
                            parents.push(idx);
                        }
                    }
                }
                let np = parents.len() as f64;
                debug!("  {} parents: {}", np, parents.iter().map(|idx| format!("{}", idx)).collect::<Vec<String>>().join(", "));
                let mut key_contained_raw = 0.0;
                for parent_idx in &parents {
                    let center = sets[*parent_idx].borrow().shape.center();
                    let distance = center.distance(&centroid);
                    if distance.is_zero() {
                        warn!("  missing region penalty: {}, parent {}, distance {}, skipping", key, parent_idx, &distance);
                    } else {
                        debug!("  missing region penalty: {}, parent {}, distance {}", key, parent_idx, &distance);
                        let contrib = distance.recip() * target / np;
                        key_contained_raw += contrib.v();
                        total_contained_penalty += contrib;
                    }
                }
                per_key_contained_raw.insert(key.clone(), key_contained_raw);
                total_missing_contained += target;
            } else {
                let mut key_disjoint_raw = 0.0;
                set_idxs.iter().for_each(|idx| {
                    let set = &sets[*idx];
                    let distance = set.borrow().shape.center().distance(&centroid);
                    debug!("  missing region penalty: {}, shape {}, distance {}", key, idx, &distance);
                    let contrib = distance * target / nf;
                    key_disjoint_raw += contrib.v();
                    total_disjoint_penalty += contrib;
                });
                per_key_disjoint_raw.insert(key.clone(), key_disjoint_raw);
                total_missing_disjoint += target;
            }
        }

        if !missing_regions.is_empty() {
            debug!("missing_regions: {:?}, {:?}", total_missing_disjoint, missing_regions);
            debug!("   disjoint: total {}, unscaled penalty {}", total_missing_disjoint, total_disjoint_penalty);
            debug!("  contained: total {}, unscaled penalty {}", total_missing_contained, total_contained_penalty);
        }
        let total_disjoint_penalty_v = total_disjoint_penalty.v();
        let disjoint_scale = if total_disjoint_penalty_v > 0. {
            total_missing_disjoint / total_disjoint_penalty_v / targets.total_area
        } else { 0. };
        if total_disjoint_penalty_v > 0. {
            total_disjoint_penalty = total_disjoint_penalty * disjoint_scale;
            debug!("  total_disjoint_penalty: {}", total_disjoint_penalty);
            if penalty_config.disjoint > 0.0 {
                error += Dual::new(0., total_disjoint_penalty.d()) * penalty_config.disjoint;
            }
        }
        let total_contained_penalty_v = total_contained_penalty.v();
        let contained_scale = if total_contained_penalty_v > 0. {
            total_missing_contained / total_contained_penalty_v / targets.total_area
        } else { 0. };
        if total_contained_penalty_v > 0. {
            total_contained_penalty = total_contained_penalty * contained_scale;
            debug!("  total_contained_penalty: {}", total_contained_penalty);
            if penalty_config.contained > 0.0 {
                error += Dual::new(0., total_contained_penalty.d()) * penalty_config.contained;
            }
        }

        // Fragmentation penalty: for region keys with multiple disconnected geometric
        // components, penalize all but the largest (L1 in fragment area).
        // Normalized by total_area so values are fractions, comparable to area errors.
        let total_area_v = total_area.v();
        let mut total_fragmentation_penalty = scene.zero();
        let mut region_penalties: BTreeMap<String, RegionPenalties> = BTreeMap::new();
        {
            let mut regions_by_key: BTreeMap<String, Vec<Dual>> = BTreeMap::new();
            for component in &scene.components {
                for region in &component.regions {
                    regions_by_key.entry(region.key.clone()).or_default().push(region.area());
                }
            }
            for (key, mut geo_areas) in regions_by_key {
                let fragment_count = geo_areas.len();
                if fragment_count > 1 {
                    geo_areas.sort_by(|a, b| b.v().partial_cmp(&a.v()).unwrap());
                    let mut key_penalty = 0.0;
                    for frag in geo_areas.into_iter().skip(1) {
                        debug!("  fragmentation penalty: key={}, fragment area={}", key, frag.v());
                        key_penalty += frag.v() / total_area_v;
                        total_fragmentation_penalty += frag;
                    }
                    let rp = region_penalties.entry(key).or_default();
                    rp.fragmentation = key_penalty;
                    rp.fragment_count = fragment_count;
                }
            }
        }
        let normalized_frag = total_fragmentation_penalty / &total_area;
        let fragmentation_v = normalized_frag.v();
        if fragmentation_v > 0.0 {
            debug!("  total_fragmentation_penalty (normalized): {}", fragmentation_v);
            if penalty_config.fragmentation > 0.0 {
                error += Dual::new(0., normalized_frag.d()) * penalty_config.fragmentation;
            }
        }

        // Merge per-key disjoint/contained penalties (apply same normalization scale)
        for (key, raw_v) in &per_key_disjoint_raw {
            region_penalties.entry(key.clone()).or_default().disjoint = raw_v * disjoint_scale;
        }
        for (key, raw_v) in &per_key_contained_raw {
            region_penalties.entry(key.clone()).or_default().contained = raw_v * contained_scale;
        }

        // Add polygon regularization penalties (self-intersection, convexity, edge regularity, perimeter:area)
        // Self-intersection and regularity are in coordinate-space units → normalize by total_area.
        // Perimeter:area is already dimensionless (P²/(4πA) - 1) → no normalization needed.
        let mut total_self_intersection_penalty = scene.zero();
        let mut total_regularity_penalty = scene.zero();
        let mut total_perimeter_area_penalty = scene.zero();
        let mut per_shape_raw_si: Vec<f64> = Vec::with_capacity(sets.len());
        let mut per_shape_raw_reg: Vec<f64> = Vec::with_capacity(sets.len());
        let mut per_shape_pa: Vec<f64> = Vec::with_capacity(sets.len());
        for set in sets.iter() {
            if let crate::shape::Shape::Polygon(poly) = &set.borrow().shape {
                // Self-intersection penalty (cross-product depth, area-ish units)
                let self_int_penalty = poly.self_intersection_penalty_dual();
                let si_v = self_int_penalty.v();
                per_shape_raw_si.push(si_v);
                if si_v > 0.0 {
                    debug!("  self-intersection penalty (raw): {}", si_v);
                    total_self_intersection_penalty = total_self_intersection_penalty + self_int_penalty;
                }

                // Regularity penalty (edge variance + convexity, area-ish units)
                let reg_penalty = poly.regularity_penalty();
                let reg_v = reg_penalty.v();
                per_shape_raw_reg.push(reg_v);
                if reg_v > 0.0 {
                    debug!("  regularity penalty (raw): {}", reg_v);
                    total_regularity_penalty = total_regularity_penalty + reg_penalty;
                }

                // Perimeter:area ratio penalty (dimensionless)
                let pa_penalty = poly.perimeter_area_penalty();
                let pa_v = pa_penalty.v();
                per_shape_pa.push(pa_v);
                if pa_v > 0.0 {
                    debug!("  perimeter:area penalty: {}", pa_v);
                    total_perimeter_area_penalty = total_perimeter_area_penalty + pa_penalty;
                }
            } else {
                per_shape_raw_si.push(0.0);
                per_shape_raw_reg.push(0.0);
                per_shape_pa.push(0.0);
            }
        }

        // Normalize coordinate-space penalties by total_area (quotient rule preserves gradients)
        let normalized_self_int = total_self_intersection_penalty / &total_area;
        let normalized_regularity = total_regularity_penalty / &total_area;

        // Build per-shape with normalized values
        let per_shape: Vec<ShapePenalties> = (0..sets.len()).map(|i| ShapePenalties {
            self_intersection: per_shape_raw_si[i] / total_area_v,
            regularity: per_shape_raw_reg[i] / total_area_v,
            perimeter_area: per_shape_pa[i],
        }).collect();

        let self_int_v = normalized_self_int.v();
        let regularity_v = normalized_regularity.v();
        let perimeter_area_v = total_perimeter_area_penalty.v();
        if self_int_v > 0.0 {
            debug!("  total_self_intersection_penalty (normalized): {}", self_int_v);
            if penalty_config.self_intersection > 0.0 {
                error += Dual::new(0., normalized_self_int.d()) * penalty_config.self_intersection;
            }
        }
        if regularity_v > 0.0 {
            debug!("  total_regularity_penalty (normalized): {}", regularity_v);
            if penalty_config.regularity > 0.0 {
                error += Dual::new(0., normalized_regularity.d()) * penalty_config.regularity;
            }
        }
        if perimeter_area_v > 0.0 {
            debug!("  total_perimeter_area_penalty: {}", perimeter_area_v);
            if penalty_config.shape_perimeter_area > 0.0 {
                error += Dual::new(0., total_perimeter_area_penalty.d()) * penalty_config.shape_perimeter_area;
            }
        }

        // Per-DR perimeter:area penalty (isoperimetric ratio P²/(4πA) - 1)
        // Fully differentiable: boundary_length_dual() returns Dual, so optimizer gets gradient signal.
        let mut total_region_pa_penalty = scene.zero();
        let mut region_perimeter_area_v = 0.0;
        if penalty_config.region_perimeter_area > 0.0 {
            for component in &scene.components {
                for region in &component.regions {
                    let area = region.area();
                    let area_v: f64 = area.clone().into();
                    if area_v < 1e-12 { continue; }
                    let perimeter = region.segments.iter()
                        .map(|s| s.edge.borrow().boundary_length_dual())
                        .reduce(|a, b| a + b)
                        .unwrap_or_else(|| scene.zero());
                    let p_sq = perimeter.clone() * &perimeter;
                    let four_pi_a = area * (4.0 * std::f64::consts::PI);
                    let ratio = p_sq / &four_pi_a;
                    let iso_v = ratio.v() - 1.0;
                    if iso_v > 0.0 {
                        let rp = region_penalties.entry(region.key.clone()).or_default();
                        rp.perimeter_area = iso_v;
                        // Area-weight (f64 scalar) so thin slivers don't dominate the total
                        let weighted = (ratio - 1.0) * (area_v / total_area_v);
                        region_perimeter_area_v += iso_v * (area_v / total_area_v);
                        total_region_pa_penalty = total_region_pa_penalty + weighted;
                    }
                }
            }
        }
        if region_perimeter_area_v > 0.0 && penalty_config.region_perimeter_area > 0.0 {
            error += Dual::new(0., total_region_pa_penalty.d()) * penalty_config.region_perimeter_area;
        }

        let penalties = Penalties {
            disjoint: total_disjoint_penalty.v(),
            contained: total_contained_penalty.v(),
            self_intersection: self_int_v,
            regularity: regularity_v,
            fragmentation: fragmentation_v,
            perimeter_area: perimeter_area_v,
            region_perimeter_area: region_perimeter_area_v,
            region_penalties,
            per_shape,
        };

        // Take shapes back from `scene`
        let shapes = sets.iter().map(|s| s.borrow().to_owned().shape).collect::<Vec<Shape<D>>>();

        // Compute gradient anchors (project error gradient onto geometric anchor points)
        let error_d = error.d();
        let gradient_anchors: Vec<Vec<GradientAnchor>> = shapes.iter()
            .map(|s| shape_gradient_anchors(s, &error_d))
            .collect();

        let converged = error.v() + penalties.total() < CONVERGENCE_THRESHOLD;
        debug!("all-in error: {:?}, converged: {}", error, converged);
        Ok(Step { shapes, components, targets, total_area, errors, error, converged, penalties, penalty_config, gradient_anchors })
    }

    pub fn n(&self) -> usize {
        self.shapes.len()
    }

    pub fn grad_size(&self) -> usize {
        self.error.1
    }

    pub fn compute_errors(scene: &Scene<D>, targets: &Targets<f64>, total_area: &Dual) -> Errors {
        let none_key = targets.none_key();
        targets.iter().filter_map(|(key, target_area)| {
            if key == &none_key {
                None
            } else {
                let actual_area = scene.area(key);
                let target_frac = target_area / targets.total_area;
                let actual_frac = actual_area.clone().unwrap_or_else(|| scene.zero()).clone() / total_area;
                let error = actual_frac.clone() - target_frac;
                let actual_frac_v = actual_frac.v();
                let kind = if target_frac > 0.0 && (actual_area.is_none() || actual_frac_v < 1e-12) {
                    ErrorKind::MissingRegion { target_frac }
                } else if target_frac < 1e-12 && actual_frac_v > 1e-12 {
                    ErrorKind::ExtraRegion { actual_frac: actual_frac_v }
                } else {
                    ErrorKind::AreaMismatch { signed_error: error.v() }
                };
                Some((
                    key.clone(),
                    Error {
                        key: key.clone(),
                        actual_area: actual_area.map(|a| a.v()),
                        target_area: *target_area,
                        actual_frac: actual_frac_v,
                        target_frac,
                        error,
                        kind,
                    }
                ))
            }
        }).collect()
    }

    // pub fn duals(&self) -> Vec<Vec<InitDual>> {
        // self.shapes.iter().map(|(_, duals)| duals.clone()).collect()
    // }

    pub fn step(&self, max_step_error_ratio: f64) -> Result<Step, SceneError> {
        let error = self.error.clone();
        // let error = self.errors.values().into_iter().map(|e| e.error.clone() * &e.error).sum::<D>().sqrt();
        let error_size = &error.v();
        let grad_vec = (-error.clone()).d();

        let step_size = error_size * max_step_error_ratio;
        let magnitude = grad_vec.iter().map(|d| d * d).sum::<f64>().sqrt();

        // If magnitude is zero (no gradient), skip the step and return a clone with same shapes
        if magnitude == 0. || magnitude.is_nan() {
            debug!("  skipping step: magnitude is {} (error_size: {}, grad_vec: {:?})", magnitude, error_size, grad_vec);
            return Step::nxt(self.shapes.clone(), self.targets.clone(), self.penalty_config.clone());
        }

        let grad_scale = step_size / magnitude;
        let step_vec = grad_vec.iter().map(|grad| grad * grad_scale).collect::<Vec<f64>>();

        debug!("  err {:?}", error);
        debug!("  step_size {}, magnitude {}, grad_scale {}", step_size, magnitude, grad_scale);
        debug!("  step_vec {:?}", step_vec);
        let shapes = &self.shapes;
        let new_shapes = shapes.iter().map(|s| s.step(&step_vec)).collect::<Vec<Shape<D>>>();
        for (cur, nxt) in shapes.iter().zip(new_shapes.iter()) {
            debug!("  {} -> {:?}", cur.v(), nxt.v());
        }
        Step::nxt(new_shapes, self.targets.clone(), self.penalty_config.clone())
    }

    /// Take an optimization step using Adam optimizer.
    ///
    /// Unlike vanilla gradient descent which uses `step_size = error * max_step_error_ratio`,
    /// Adam maintains per-parameter momentum and variance estimates, enabling:
    /// - Escape from local minima via momentum
    /// - Adaptive per-parameter learning rates
    /// - Smoother convergence with less oscillation
    pub fn step_with_adam(&self, adam: &mut AdamState, learning_rate: f64) -> Result<Step, SceneError> {
        let error = self.error.clone();
        let grad_vec = (-error.clone()).d();

        let magnitude = grad_vec.iter().map(|d| d * d).sum::<f64>().sqrt();

        // If magnitude is zero (no gradient), skip the step and return a clone with same shapes
        if magnitude == 0. || magnitude.is_nan() {
            debug!("  skipping step: magnitude is {} (grad_vec: {:?})", magnitude, grad_vec);
            return Step::nxt(self.shapes.clone(), self.targets.clone(), self.penalty_config.clone());
        }

        // Adam computes the step vector using momentum and variance estimates
        let step_vec = adam.step(&grad_vec, learning_rate);

        debug!("  err {:?} (Adam step {})", error, adam.t);
        debug!("  learning_rate {}, magnitude {}", learning_rate, magnitude);
        debug!("  grad_vec {:?}", grad_vec);
        debug!("  step_vec {:?}", step_vec);

        let shapes = &self.shapes;
        let new_shapes = shapes.iter().map(|s| s.step(&step_vec)).collect::<Vec<Shape<D>>>();
        for (cur, nxt) in shapes.iter().zip(new_shapes.iter()) {
            debug!("  {} -> {:?}", cur.v(), nxt.v());
        }
        Step::nxt(new_shapes, self.targets.clone(), self.penalty_config.clone())
    }

    /// Take an optimization step with gradient clipping.
    ///
    /// Unlike vanilla `step()` which can take very large steps when error is high,
    /// this method clips gradients to prevent oscillation:
    /// - Per-component clipping: each gradient is clamped to [-max_grad, max_grad]
    /// - L2 norm clipping: total gradient magnitude is clamped to max_grad_norm
    /// - Fixed learning rate: doesn't scale with error, providing more stable updates
    ///
    /// This is a simpler alternative to Adam that doesn't require persistent state.
    pub fn step_clipped(&self, learning_rate: f64, max_grad_value: f64, max_grad_norm: f64) -> Result<Step, SceneError> {
        let error = self.error.clone();
        let grad_vec = (-error.clone()).d();

        let magnitude = grad_vec.iter().map(|d| d * d).sum::<f64>().sqrt();

        // If magnitude is zero (no gradient), skip the step
        if magnitude == 0. || magnitude.is_nan() {
            debug!("  skipping step: magnitude is {} (grad_vec: {:?})", magnitude, grad_vec);
            return Step::nxt(self.shapes.clone(), self.targets.clone(), self.penalty_config.clone());
        }

        // Clip by value (per-component)
        let mut clipped: Vec<f64> = grad_vec.iter()
            .map(|&g| g.clamp(-max_grad_value, max_grad_value))
            .collect();

        // Clip by L2 norm
        let clipped_norm: f64 = clipped.iter().map(|g| g * g).sum::<f64>().sqrt();
        if clipped_norm > max_grad_norm {
            let scale = max_grad_norm / clipped_norm;
            for g in &mut clipped {
                *g *= scale;
            }
        }

        // Apply fixed learning rate (not scaled by error)
        let step_vec: Vec<f64> = clipped.iter().map(|&g| g * learning_rate).collect();

        debug!("  err {:?} (clipped step)", error);
        debug!("  learning_rate {}, original magnitude {}, clipped norm {}", learning_rate, magnitude, clipped_norm.min(max_grad_norm));
        debug!("  step_vec {:?}", step_vec);

        let shapes = &self.shapes;
        let new_shapes = shapes.iter().map(|s| s.step(&step_vec)).collect::<Vec<Shape<D>>>();
        Step::nxt(new_shapes, self.targets.clone(), self.penalty_config.clone())
    }
}
