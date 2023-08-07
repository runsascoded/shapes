use std::{f64::consts::PI, ops::{Add, Mul, Deref, Div}};

use autodiff::*;

//type F3 = (f64, f64, f64);
#[derive(Debug)]
pub struct F3([f64; 3]);

#[derive(Debug)]
pub struct D(F<f64, F3>);

impl Deref for F3 {
    type Target = [f64; 3];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for D {
    type Target = F<f64, F3>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// pub trait Z(Zero);

impl Add for F3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        F3([self[0] + rhs[0], self[1] + rhs[1], self[2] + rhs[2]])
    }
}

impl Zero for F3 {
    fn zero() -> Self {
        F3([0.0, 0.0, 0.0])
    }

    fn is_zero(&self) -> bool {
        self[0] == 0.0 && self[1] == 0.0 && self[2] == 0.0
    }
}

// impl Mul for F3 {
//     type Output = Self;
//     fn mul(self, rhs: Self) -> Self {
//         F3([self[0] * rhs[0], self[1] * rhs[1], self[2] * rhs[2]])
//     }
// }

impl Mul<f64> for F3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        F3([self[0] * rhs, self[1] * rhs, self[2] * rhs])
    }
}

impl Div<f64> for F3 {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        F3([self[0] / rhs, self[1] / rhs, self[2] / rhs])
    }
}

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

impl FromFloat for D {
    fn from<T>(f: f64) -> Self {
        D(F::cst(f))
    }
}

pub struct Point<T> {
    x: T,
    y: T,
}

pub struct Circle<T> {
    c: Point<T>,
    r: T,
}

impl<T: Mul<T, Output = T> + Mul<f64, Output = T> + FromFloat> Circle<T>
{
    pub fn area(&self) -> T {
        // let pi: T = FromFloat::from::<T>(PI);
        let pi_r: T = self.r * PI;
        pi_r * self.r
    }
}

// use autodiff::F::var;

fn main() {
    // let var = F::var;
    let z = F3([0.0, 0.0, 0.0]);
    let r: D = D(F::new(1.0, z));
    let x: D = D(F::new(0.0, z));
    let y: D = D(F::new(0.0, z));
    let c: Point<D> = Point { x, y };
    let c1 = Circle {
        r,
        c,
     };
    // let c2: Circle<FT<f64>> = Circle {
    //     r: F::new(1.0, F3([1, 0, 0])),
    //     c: Point {
    //         x: F::new(1.0, F3([0, 1, 0])),
    //         y: F::new(1.0, F3([0, 0, 1])),
    //      },
    //  };
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
