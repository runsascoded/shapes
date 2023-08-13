use crate::circle::Circle;

#[derive(Clone)]
pub struct Intersection<'a, D> {
    pub x: &'a D,
    pub y: &'a D,
    pub c1: &'a Circle<D>,
    pub c2: &'a Circle<D>,
}