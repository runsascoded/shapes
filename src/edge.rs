use crate::{circle::Circle, intersection::Intersection};


pub struct Edge<'a, D> {
    pub c: Circle<'a, D>,
    pub intersections: [ Intersection<'a, D>; 2 ],
}