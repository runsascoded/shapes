use crate::{shape::Shape::{self}, transform::{CanProject, HasProjection, CanTransform}, ellipses::{xyrrt::LevelArg, xyrr::UnitCircleGap}, geometry::polygon};

pub trait Gap<O> {
    type Output;
    fn gap(&self, o: &O) -> Option<Self::Output>;
}

impl<
    D: LevelArg + UnitCircleGap + polygon::UnitCircleGap
> Gap<Shape<D>> for Shape<D>
where
    Shape<D>: CanTransform<D, Output = Shape<D>> + HasProjection<D>,
{
    type Output = D;
    fn gap(&self, o: &Shape<D>) -> Option<D> {
        self.apply(&o.projection()).unit_circle_gap()
    }
}