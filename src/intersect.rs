use crate::{intersection::Intersection, circle::Circle, dual::D, ellipses::xyrr::XYRR};

trait Intersect<O> {
    fn intersect(&self, o: &O) -> Vec<Intersection>;
}

impl Intersect<Circle<D>> for Circle<D> {
    fn intersect(&self, o: &Circle<D>) -> Vec<Intersection> {
        self.intersect(o)
    }
}

impl Intersect<XYRR<D>> for Circle<D> {
    fn intersect(&self, o: &XYRR<D>) -> Vec<Intersection> {
        self.intersect(o)
    }
}