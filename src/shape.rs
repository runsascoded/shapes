use std::{rc::Rc, cell::RefCell};

use derive_more::{From, Into};
use tsify::declare;

use crate::{dual::D, circle, ellipses::xyrr, zero::Zero, transform::{Projection, Transform}, r2::R2, intersection::Intersection};

#[declare]
pub type Duals = Vec<Vec<f64>>;
#[declare]
pub type Input = (Shape<f64>, Duals);

#[derive(Debug, Clone, From)]
pub enum Shape<D> {
    Circle(circle::Circle<D>),
    XYRR(xyrr::XYRR<D>),
}

impl Shape<f64> {
    pub fn dual(&self, duals: &Duals) -> Shape<D> {
        match self {
            Shape::Circle(c) => Shape::Circle(c.dual(duals)),
            Shape::XYRR(e) => Shape::XYRR(e.dual(duals)),
        }
    }
}

impl<D> Shape<D> {
    pub fn idx(&self) -> usize {
        match self {
            Shape::Circle(c) => c.idx,
            Shape::XYRR(e) => e.idx,
        }
    }
}

impl Shape<D> {
    pub fn n(&self) -> usize {
        match self {
            Shape::Circle(c) => c.n(),
            Shape::XYRR(e) => e.n(),
        }
    }
    pub fn zero(&self) -> D {
        match self {
            Shape::Circle(c) => Zero::zero(&c.r),
            Shape::XYRR(e) => Zero::zero(&e.c.x),
        }
    }
    // pub fn intersect(&self, other: &Shape<D>) -> Vec<R2<D>> {
    //     match (self, other) {
    //         (Shape::Circle(c0), Shape::Circle(c1)) => c0.intersect(&c1),
    //         (Shape::XYRR(e), other) => e.intersect(&other),
    //         (other, Shape::XYRR(e)) => e.intersect(&other),
    //     }
    // }
    pub fn apply(&self, projection: &Projection<D>) -> Shape<D> {
        projection.0.iter().fold(*self, |c, t| c.transform(t))
    }
    pub fn transform(&self, transform: &Transform<D>) -> Shape<D> {
        match self {
            Shape::Circle(c) => c.transform(transform),
            Shape::XYRR(e) => Shape::XYRR(e.transform(transform)),
        }
    }
    pub fn projection(&self) -> Projection<D> {
        match self {
            Shape::Circle(c) => c.projection(),
            Shape::XYRR(e) => e.projection(),
        }
    }
    pub fn project(&self, o: &Shape<D>) -> Self {
        let projection = self.projection();
        o.apply(&projection)
    }
    pub fn intersect(&self, o: &Shape<D>) -> Vec<Intersection> {
        let c0 = self;
        let projection = self.projection();
        let projected = o.apply(&projection);
        let unit_intersections = projected.unit_intersections();
        let points = unit_intersections.iter().map(|p| p.transform(-projection));
        let intersections = points.map(|p| {
            let x = p.x.clone();
            let y = p.y.clone();
            let p = R2 { x: x.clone(), y: y.clone() };
            let t0 = c0.theta(p.clone());
            let t1 = o.theta(p.clone());
            Intersection { x, y, c0idx: c0.idx(), c1idx: o.idx(), t0, t1, }
        });
        intersections.collect()
    }
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        match self {
            Shape::Circle(c) => c.unit_intersections(),
            Shape::XYRR(e) => e.unit_intersections(),
        }
    }
    pub fn theta(&self, p: R2<D>) -> D {
        match self {
            Shape::Circle(c) => c.theta(p),
            Shape::XYRR(e) => e.theta(p),
        }
    }
}

pub type S = Rc<RefCell<Shape<D>>>;
