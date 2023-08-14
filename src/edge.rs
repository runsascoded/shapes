use crate::{circle::Circle, intersection::Intersection};


pub struct Edge<D> {
    pub c: Circle<D>,
    pub intersections: [ Intersection<D>; 2 ],
}