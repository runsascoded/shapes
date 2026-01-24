use std::{ops::Sub, iter::Sum, fmt};

use approx::{AbsDiffEq, RelativeEq};
use derive_more::Deref;
use itertools::Itertools;
use log::debug;
use ordered_float::OrderedFloat;

use crate::dual::Dual;

use super::complex::{Complex, self, Norm};

#[derive(Clone, Debug, Deref, PartialEq)]
pub struct Roots<D>(pub Vec<Complex<D>>);

pub trait Align
: Norm
+ fmt::Debug
+ Into<f64>
+ Sum
+ Sub<Output = Self>
{}
impl Align for f64 {}
impl Align for Dual {}

impl<D: Align> Roots<D> {
    pub fn align(&self, other: &Roots<D>) -> Option<Self> {
        if self.len() != other.len() {
            None
        } else {
            let permutations = self.iter().permutations(self.len()).map(|perm| {
                let total_distance = perm.clone().into_iter().zip(other.iter()).map(|(c0, c1)| (c0.clone() - c1.clone()).norm()).sum::<D>();
                (total_distance, perm)
            });
            let min = permutations.min_by_key(|(total_distance, _)| OrderedFloat(total_distance.clone().into()));
            min.map(|(distance, roots)| {
                debug!("aligned: {}", distance.into());
                debug!("  {:?}", roots);
                debug!("  {:?}", other);
                Roots(roots.into_iter().cloned().collect())
            })
        }
    }
}

impl<D: Align + complex::Eq> AbsDiffEq for Roots<D>
{
    type Epsilon = D::Epsilon;
    fn default_epsilon() -> Self::Epsilon {
        D::default_epsilon()
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        match self.align(other) {
            None => {
                debug!("abs_diff_eq: roots not aligned: {} vs {}", self.len(), other.len());
                false
            },
            Some(roots) => {
                let rv = roots.0.abs_diff_eq(&other.0, epsilon);
                if !rv {
                    debug!("abs_diff_eq: roots not equal: {:?} vs {:?}", roots, other);
                }
                rv
            }
        }
    }
}

impl<D: Align + complex::Eq> RelativeEq for Roots<D>
{
    fn default_max_relative() -> Self::Epsilon {
        D::default_max_relative()
    }
    fn relative_eq(&self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon) -> bool {
        match self.align(other) {
            None => {
                debug!("relative_eq: roots not aligned: {} vs {}", self.len(), other.len());
                false
            },
            Some(roots) => {
                let rv = roots.0.relative_eq(&other.0, epsilon, max_relative);
                if !rv {
                    debug!("relative_eq: roots not equal:");
                    debug!("  {:?}", roots);
                    debug!("  {:?}", other);
                }
                rv
            }
        }
    }
}