use std::f64::consts::PI;

use autodiff::*;

// trait Area {
//     fn area<T: Float>(&self) -> T;
// }

pub trait FromFloat {
    fn from<T>(f: f64) -> Self;
}

impl FromFloat for f64 {
    fn from<T>(f: f64) -> Self {
        f
    }
}

impl FromFloat for FT<f64> {
    fn from<T>(f: f64) -> Self {
        F::cst(f)
    }
}

pub struct Point<T: Float> {
    x: T,
    y: T,
}

pub struct Circle<T: Float> {
    c: Point<T>,
    r: T,
}

impl<T: Float + FromFloat> Circle<T> {
    pub fn area(&self) -> T {
        let pi: T = FromFloat::from::<T>(PI);
        pi * self.r * self.r
    }
}

fn main() {
    let c1 = Circle { r: 1.0, c: Point { x: 2.0, y: 3.0 } };
    let c2: Circle<FT<f64>> = Circle { r: F::var(2.0), c: Point { x:F::var(-1.0), y: F::var(-1.0) } };
    // let c3 = Circle { r: 2 };

    dbg!("c1: {}", c1.area());
    dbg!("c2: {}", c2.area());

    // let circle_area = |r: FT<f64>| PI * r * r;
    // let r = F::var(1.0);
    // let area = circle_area(r);
    // dbg!("value: {}, deriv: {}", area.value(), area.deriv());

    // let grad = area.grad();
    // dbg!("area: {}", area);
    // dbg!("grad: {}", grad);
    // FT::from(na::Vector2::new(1.0, 2.0));
    // dbg!(FT::from(1.0));
    // let r = FT::from(1.0);
    // let area = circle_area(r);
    // println!("area: {}", area);
    // println!("Hello, world!");
}
