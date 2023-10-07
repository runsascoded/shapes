use std::{fmt::{Formatter, Display, self}, ops::{Add, Sub}, iter::Sum, collections::BTreeSet, f64::consts::TAU, rc::Rc, cell::RefCell};

use log::debug;

use crate::{segment::Segment, edge::{Edge, EdgeArg}, math::abs::{Abs, AbsArg}, r2::R2, to::To, dual::Dual, component::C, contains::{Contains, ShapeContainsPoint}, transform::{CanProject, HasProjection}, shape::Shape, theta_points::{ThetaPoints, ThetaPointsArg}};

#[derive(Debug, Clone)]
pub struct Region<D> {
    pub key: String,
    pub segments: Vec<Segment<D>>,
    pub container_set_idxs: BTreeSet<usize>,
    pub child_components: Vec<C<D>>,
}

pub type R<D> = Rc<RefCell<Region<D>>>;

pub trait RegionArg
: EdgeArg
+ AbsArg
+ Sum
+ Add<Output = Self>
+ Sub<Output = Self>
{}
impl RegionArg for f64 {}
impl RegionArg for Dual {}

impl<D: RegionArg> Region<D>
where R2<D>: To<R2<f64>>,
{
    pub fn len(&self) -> usize {
        self.segments.len()
    }
    pub fn polygon_area(&self) -> D {
        (self.segments.iter().map(|s| {
            let cur = s.start().borrow().p.clone();
            let nxt = s.end().borrow().p.clone();
            cur.x * nxt.y - cur.y * nxt.x
        }).sum::<D>() / 2.).abs()
    }
    pub fn secant_area(&self) -> D {
        let is_singleton = self.segments.len() == 1;
        self.segments.iter().map(|s| {
            let area = s.secant_area();
            let set_idx = s.edge.borrow().set_idx();
            // debug!("  secant_area: {}, set_idx: {}, container_set_idxs {:?}", area, set_idx, self.container_set_idxs);
            if is_singleton || self.container_set_idxs.contains(&set_idx) { area } else { -area }
        }).sum::<D>()
    }
    /// Area of this region (including any child components)
    pub fn total_area(&self) -> D {
        let polygon_area = self.polygon_area();
        let secant_area = self.secant_area();
        let area = polygon_area.clone() + secant_area.clone();
        // debug!("Region {}: polygon_area: {}, secant_area: {}, total: {}", self.key, polygon_area, secant_area, area);
        area
    }
    /// Area of this region (excluding any child components)
    pub fn area(&self) -> D {
        let mut area = self.total_area();
        for child_component in &self.child_components {
            // TODO: implement SubAssign
            area = area - child_component.borrow().area();
        }
        area
    }
    pub fn matches(&self, key: &String) -> bool {
        for (idx, ch) in (&key).chars().enumerate() {
            let is_container = self.container_set_idxs.contains(&idx);
            if ch == '-' && is_container {
                return false;
            }
            if ch != '*' && !is_container {
                return false
            }
        }
        true
    }
}

pub trait RegionContainsArg
: ShapeContainsPoint
+ EdgeArg
+ ThetaPointsArg
{}
impl RegionContainsArg for f64 {}
impl RegionContainsArg for Dual {}

impl<D: RegionContainsArg> Region<D>
where
    // ShapeContainsPoint
    R2<D>: CanProject<D, Output = R2<D>>,
    Shape<D>: HasProjection<D>,
    // EdgeArg
    R2<D>: To<R2<f64>>,
{
    pub fn contains(&self, p: &R2<D>) -> bool {
        for segment in &self.segments {
            let edge = segment.edge.clone();
            let edge_set = edge.borrow().set.clone();
            let edge_set_contains_region = self.container_set_idxs.contains(&edge_set.borrow().idx);
            let shape = &edge_set.borrow().shape;
            let edge_set_contains_point = shape.contains(p);
            // debug!(
                // "  region {}: segment {} vs point {}: edge_set_contains_region: {}, edge_set_contains_point: {}",
                // self.key, segment, p,
                // edge_set_contains_region, edge_set_contains_point,
            // );
            if edge_set_contains_region != edge_set_contains_point {
                return false;
            }
            let start = edge.borrow().node0.borrow().p.clone();
            let end = edge.borrow().node1.borrow().p.clone();
            let start_theta: f64 = shape.theta(&start).into();
            let mut end_theta: f64 = shape.theta(&end).into();
            if start_theta >= end_theta {
                end_theta += TAU;
            }
            let mut theta: f64 = shape.theta(p).into();
            if theta < start_theta {
                theta += TAU;
            }
            // debug!("  start_theta: {}, end_theta: {}, theta: {}", start_theta, end_theta, theta);
            if theta < start_theta || theta > end_theta {
                return false;
            }
        }
        return true
    }
}

impl<D: Display> Display for Region<D>
where
    Edge<D>: Display
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f, "R({}\n\t{}\n)",
            self.container_set_idxs.iter().map(|i| {
                format!("{}", i)
            }).collect::<Vec<String>>().join(", "),
            self.segments.iter().map(|s| {
                format!("{}", s)
            }).collect::<Vec<String>>().join(",\n\t")
        )
    }
}