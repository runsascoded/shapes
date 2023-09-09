// use std::ops::Add;

use std::fmt::{Display, Formatter, self};

use derive_more::{Add, Sub, Mul, Div, Neg};


#[derive(Clone, Debug, Add, Sub, Mul, Div, Neg)]
pub struct Complex<D> {
    pub re: D,
    pub im: D,
}

impl<D: Display> Display for Complex<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.6} + {:.6}i", self.re, self.im)
    }
}

pub type ComplexPair<D> = Complex<D>;

// impl<D: Add<Output = D>> Add for Complex<D> {
//     type Output = Self;
//     fn add(self, rhs: Self) -> Self::Output {
//         Complex {
//             re: self.re + rhs.re,
//             im: self.im + rhs.im,
//         }
//     }
// }