use crate::{shape::{InputSpec, Shape, Duals}, dual::Dual};

pub static Z: bool = false;
pub static D: bool = true;

/// Placeholder for the derivative component of a [`Dual`] (corresponding to one "coordinate" of a Shape, e.g. `c.x`).
/// The `usize` argument indicates the length of the derivative vector, which should be the same for all coordinates in a [`Model`].
/// `Zeros` values are stateless, but `OneHot`s are expanded during [`Model`]/[`Step`] construction, so that the "hot" element proceeds through the vector,
/// i.e. the first differentiable coordinate will be `one_hot(0)`, the second will be `one_hot(1)`, etc.
/// This level of indirection makes it easier to specify a model's Shapes, and toggle which coordinates are considered moveable (and whose error-partial-derivative ∂(error) is propagated through all calculations).
/// See [`model::tests`] for example:
/// ```rust
/// use crate::dual::{d, z};
/// let inputs = vec![
///     (circle(0., 0., 1.), vec![ Z, Z, Z ]),
///     (circle(1., 0., 1.), vec![ D, Z, D ]),
/// ];
/// let targets = [
///     ("0*", 1. /  3.),  // Fizz (multiples of 3)
///     ("*1", 1. /  5.),  // Buzz (multiples of 5)
///     ("01", 1. / 15.),  // Fizz Buzz (multiples of both 3 and 5)
/// ];
/// let model = Model::new(inputs, targets.to());
/// ```
/// This initializes two [`Circle`]s (with f64 coordinate values), such that
pub struct InitDuals {
    pub nxt: usize,
    pub n: usize
}
impl InitDuals {
    pub fn from<const N: usize>(input_specs: [InputSpec; N]) -> [ Shape<Dual>; N ] {
        let n = input_specs.iter().map(|(_, spec)| spec.iter().filter(|v| **v).count()).sum();
        let mut init = InitDuals::new(n);
        input_specs.map(|spec| init.shape(&spec))
    }
    pub fn from_vec(input_specs: &Vec<InputSpec>) -> Vec<Shape<Dual>> {
        let n = input_specs.iter().map(|(_, spec)| spec.iter().filter(|v| **v).count()).sum();
        let mut init = InitDuals::new(n);
        input_specs.iter().map(|spec| init.shape(spec)).collect()
    }
    pub fn new(n: usize) -> Self {
        InitDuals { nxt: 0, n }
    }
    pub fn next(&mut self, differentiable: bool) -> Vec<f64> {
        if differentiable {
            let duals = one_hot(&self.nxt, &self.n);
            self.nxt += 1;
            duals
        } else {
            vec![0.; self.n]
        }
    }
    pub fn shape(&mut self, (s, init_duals): &InputSpec) -> Shape<Dual> {
        let duals: Duals = init_duals.iter().map(|init_dual| self.next(*init_dual)).collect();
        s.dual(&duals)
    }
}

pub fn one_hot(idx: &usize, size: &usize) -> Vec<f64> {
    let mut v = vec![0.; *size];
    v[*idx] = 1.;
    v
}

pub fn is_one_hot(v: &Vec<f64>) -> Option<usize> {
    let mut idx = None;
    for (i, x) in v.iter().enumerate() {
        if *x == 1. {
            if idx.is_some() {
                return None;
            }
            idx = Some(i);
        } else if *x != 0. {
            return None;
        }
    }
    idx
}
