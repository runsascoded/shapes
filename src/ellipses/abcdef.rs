use crate::{r2::R2, dual::D};

use super::quartic::quartic_roots;

/// Ax² + Bxy + Cy² + Dx + Ey + F = 0
pub struct ABCDEF<D> {
    pub a: D,
    pub b: D,
    pub c: D,
    pub d: D,
    pub e: D,
    pub f: D,
}

impl ABCDEF<D> {
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        let ac1 = self.a.clone() - self.c.clone() - 1.;  // A - C - 1
        let b2 = self.b.clone() * self.b.clone();  // B²
        let d2 = self.d.clone() * self.d.clone();  // D²
        let e2 = self.e.clone() * self.e.clone();  // E²
        let f2 = self.f.clone() * self.f.clone();  // F²
        let c_4 = ac1.clone() * ac1.clone() + b2.clone();  // (A - C - 1)² + B²
        let c_3 = (self.b.clone() * self.d.clone() - self.e.clone() * ac1.clone()) * 2.;  // 2(BD - E(A - C - 1))
        let c_2 = d2.clone() - b2 - e2;  // D² - B² - E²
        let c_1 = (self.e.clone() * self.f.clone() + self.b.clone() * self.d.clone()) * -2.;  // -2(EF + BD)
        let c_0 = f2 - d2;  // F² - D²
        quartic_roots(c_4, c_3, c_2, c_1, c_0)
    }
    // pub fn rotate(&self, t: &D) -> ABCDEF<D> {
    //     todo!()
    // }
    // Rotate the plane so that this ellipse ends up aligned with the x- and y-axes (i.e. θ == B == 0)
    // pub fn level(&self) -> ACDEF<D> {
    //     todo!()
    // }
}