use std::ops::Deref;

use crate::{shape::InputSpec, step::Step};

pub struct CoordGetter<In> {
    pub name: String,
    pub get: Box<dyn Fn(In) -> f64>,
}

impl<In> Deref for CoordGetter<In> {
    type Target = Box<dyn Fn(In) -> f64>;
    fn deref(&self) -> &Self::Target {
        &self.get
    }
}

impl<In> CoordGetter<In> {
    pub fn new(name: &str, get: Box<dyn Fn(In) -> f64>) -> Self {
        Self { name: name.to_string(), get }
    }
}

impl<'a: 'static, In: 'a> CoordGetter<In> {
    pub fn map<New: 'a>(&'a self, f: Box<dyn Fn(New) -> In>) -> CoordGetter<New> {
        CoordGetter {
            name: self.name.clone(),
            get: Box::new(move |new: New| (self.get)(f(new))),
        }
    }
}

pub fn coord_getter<In, T: Fn(In) -> f64 + 'static>(name: &str, get: T) -> CoordGetter<In> {
    CoordGetter::new(name, Box::new(get))
}

#[derive(derive_more::Deref)]
pub struct CoordGetters<In>(pub Vec<CoordGetter<In>>);

impl From<Vec<InputSpec>> for CoordGetters<Step> {
    fn from(inputs: Vec<InputSpec>) -> Self {
        Self(
            inputs
            .iter()
            .enumerate()
            .flat_map(
                |(shape_idx, (shape, mask))| {
                    let getters = shape.getters(shape_idx);
                    getters.into_iter().zip(mask).filter(|(_, mask)| **mask).map(|(getter, _)| {
                        CoordGetter {
                            name: format!("{}.{}", shape_idx, getter.name),
                            get: Box::new(move |step: Step| getter(step.shapes[shape_idx].v())),
                        }
                    }).collect::<Vec<_>>()
            }).collect()
        )
    }
}