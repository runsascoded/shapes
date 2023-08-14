use crate::circle::Circle;

#[derive(Clone)]
pub struct Intersection<D> {
    pub x: D,
    pub y: D,
    pub c1: Circle<D>,
    pub c2: Circle<D>,
}