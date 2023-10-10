use crate::{segment::Segment, region::{Region, RegionArg}, r2::R2, to::To};

#[derive(Clone, Debug, derive_more::Deref)]
pub struct Hull<D>(pub Vec<Segment<D>>);

impl<D: RegionArg> Hull<D>
where R2<D>: To<R2<f64>>,
{
    pub fn area(&self) -> D {
        let polygon_area = Region::polygon_area(&self);
        let secant_area = Region::secant_area(&self);
        polygon_area + secant_area
    }
}