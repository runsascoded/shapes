use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Display, Formatter},
    iter::Sum,
    ops::{Add, Sub, SubAssign},
    rc::Rc,
};

use itertools::Itertools;
use ordered_float::OrderedFloat;

use crate::{
    component::C,
    dual::Dual,
    edge::{Edge, EdgeArg, E},
    math::abs::{Abs, AbsArg},
    r2::R2,
    segment::Segment,
    shape::Shape,
    theta_points::ThetaPoints,
    to::To,
};

#[derive(Debug, Clone)]
pub struct Region<D> {
    pub key: String,
    pub segments: Vec<Segment<D>>,
    pub container_set_idxs: BTreeSet<usize>,
    pub child_components: Vec<C<D>>,
    pub polygon_area: D,
    pub secant_area: D,
    /// Area of this region (including any child components)
    pub total_area: D,
}

pub type R<D> = Rc<RefCell<Region<D>>>;

pub trait RegionArg: EdgeArg + AbsArg + Sum + Add<Output = Self> + Sub<Output = Self> + SubAssign {}
impl RegionArg for f64 {}
impl RegionArg for Dual {}

impl<D: RegionArg> Region<D>
where
    R2<D>: To<R2<f64>>,
{
    pub fn new(
        key: String,
        segments: Vec<Segment<D>>,
        container_set_idxs: BTreeSet<usize>,
    ) -> Self {
        let polygon_area = Self::polygon_area(&segments);
        let secant_area = Self::secant_area(&segments);
        let total_area = (polygon_area.clone() + secant_area.clone()).abs();
        // debug!(
        //     "Region {} polygon_area: {}, secant_area: {}, total: {}",
        //     key,
        //     Into::<f64>::into(polygon_area.clone()),
        //     Into::<f64>::into(secant_area.clone()),
        //     Into::<f64>::into(total_area.clone())
        // );
        Region {
            key,
            segments,
            container_set_idxs,
            child_components: vec![], // populated by `Scene`, once all `Component`s have been created
            polygon_area,
            secant_area,
            total_area,
        }
    }
    pub fn len(&self) -> usize {
        self.segments.len()
    }
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
    pub fn polygon_area(segments: &[Segment<D>]) -> D {
        segments
            .iter()
            .map(|s| {
                let cur = s.start().borrow().p.clone();
                let nxt = s.end().borrow().p.clone();
                cur.x * nxt.y - cur.y * nxt.x
            })
            .sum::<D>()
            / 2.
    }
    pub fn secant_area(segments: &[Segment<D>]) -> D {
        segments
            .iter()
            .map(|s| { s.secant_area() })
            .sum::<D>()
    }
    /// Area of this region (excluding any child components)
    pub fn area(&self) -> D {
        let mut area = self.total_area.clone();
        for child_component in &self.child_components {
            area -= child_component.borrow().area();
        }
        area
    }
    pub fn matches(&self, key: &str) -> bool {
        for (idx, ch) in key.chars().enumerate() {
            let is_container = self.container_set_idxs.contains(&idx);
            if ch == '-' && is_container {
                return false;
            }
            if ch != '*' && !is_container {
                return false;
            }
        }
        true
    }
}

impl<D: fmt::Debug> Region<D> {
    pub fn segments_for_set(&self, set_idx: usize) -> Vec<&Segment<D>> {
        self.segments
            .iter()
            .filter(|s| s.edge.borrow().set_idx() == set_idx)
            .collect()
    }
    pub fn edges_for_set(&self, set_idx: usize) -> Vec<E<D>> {
        self.segments_for_set(set_idx)
            .iter()
            .map(|s| s.edge.clone())
            .collect()
    }
}

impl<D: Clone + Display + fmt::Debug + Into<f64> + PartialOrd> Region<D>
where
    Shape<f64>: From<Shape<D>>,
{
    pub fn contains(&self, p: &R2<f64>, all_shapes: &BTreeMap<usize, Shape<f64>>) -> bool {
        let y = p.y;
        let mut points_at_y: Vec<(usize, f64, f64)> = all_shapes
            .iter()
            .flat_map(|(idx, s)| {
                s.at_y(y).into_iter().map(|x| {
                    let p = R2 { x, y };
                    let theta = s.theta(&p);
                    (*idx, x, theta)
                })
            })
            .collect();
        points_at_y.sort_by_cached_key(|(_, x, _)| OrderedFloat(*x));
        let component_container_set_idxs: BTreeSet<usize> = self
            .container_set_idxs
            .iter()
            .filter(|set_idx| all_shapes.contains_key(set_idx))
            .cloned()
            .collect();
        let mut cur_set_idxs: BTreeSet<usize> = BTreeSet::new();
        // debug!("Checking containment: region {}, p: {}, shapes {}, points_at_y: {:?}", self.key, p, all_shapes.keys().join(","), points_at_y);
        for (idx, (prv, cur)) in points_at_y.into_iter().tuple_windows().enumerate() {
            let (set_idx, x, _) = cur;
            if idx == 0 {
                if p.x <= prv.1 {
                    // debug!("  point is outside first x");
                    return false;
                }
                cur_set_idxs.insert(prv.0);
            }
            if p.x <= x {
                if cur_set_idxs != component_container_set_idxs {
                    // debug!("  breaking between {} and {}: {:?} != {:?}", prv.1, x, cur_set_idxs, self.container_set_idxs);
                    return false;
                }
                let prv_edges = self.edges_for_set(prv.0);
                if !prv_edges
                    .iter().any(|edge| edge.borrow().contains_theta(prv.2))
                {
                    // debug!("  breaking between {} and {}: {} does not contain theta {}", prv.1, x, prv_edges.iter().map(|e| format!("{}", e.borrow())).join(","), prv.2);
                    return false;
                }
                let cur_edges = self.edges_for_set(cur.0);
                if !cur_edges
                    .iter().any(|edge| edge.borrow().contains_theta(cur.2))
                {
                    // debug!("  breaking between {} and {}: {} does not contain theta {}", prv.1, x, cur_edges.iter().map(|e| format!("{}", e.borrow())).join(","), cur.2);
                    return false;
                }
                return true;
            }
            if cur_set_idxs.contains(&set_idx) {
                // debug!("  removing set_idx {}", set_idx);
                cur_set_idxs.remove(&set_idx);
            } else {
                // debug!("  inserting set_idx {}", set_idx);
                cur_set_idxs.insert(set_idx);
            }
        }
        false
    }
}

impl<D: Display> Display for Region<D>
where
    Edge<D>: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "R({}\n\t{}\n)",
            self.container_set_idxs
                .iter()
                .map(|i| { format!("{}", i) })
                .collect::<Vec<String>>()
                .join(", "),
            self.segments
                .iter()
                .map(|s| { format!("{}", s) })
                .collect::<Vec<String>>()
                .join(",\n\t")
        )
    }
}
