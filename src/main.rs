use num_dual::*;
use nalgebra::SMatrix;

fn f<D: DualNum<f64> + PartialOrd + Copy>(cx: D, cy: D, r: D) -> SMatrix<D, 4, 1> {
    let cx2 = cx * cx;
    let cy2 = cy * cy;
    let C = cx2 + cy2;
    let r2 = r * r;
    let D = C - r2 + 1.;
    let a = C * 4.;
    let b = cx * D * -4.;
    let c = D*D - cy2 * 4.;

    let d = b*b - a*c*4.;
    let dsqrt = d.sqrt();
    let rhs = dsqrt / 2. / a;
    let lhs = -b / 2. / a;

    let x0 = lhs - rhs;
    let y0sq = -x0*x0 + 1.;
    let y0_0 = y0sq.sqrt();
    let y0_1 = -y0_0;
    let x0cx = x0 - cx;
    let x0cx2 = x0cx * x0cx;
    let y0_0cy = y0_0 - cy;
    let y0_1cy = y0_1 - cy;
    let check0_0 = (x0cx2 + y0_0cy*y0_0cy - r2).abs();
    let check0_1 = (x0cx2 + y0_1cy*y0_1cy - r2).abs();
    let y0 = if check0_0 < check0_1 { y0_0 } else { y0_1 };
    println!("checks0: {:?}, {:?}", check0_0, check0_1);

    let x1 = lhs + rhs;
    let y1sq = -x1*x1 + 1.;
    let y1_0 = y1sq.sqrt();
    let y1_1 = -y1_0;
    let x1cx = x1 - cx;
    let x1cx2 = x1cx * x1cx;
    let y1_0cy = y1_0 - cy;
    let y1_1cy = y1_1 - cy;
    let check1_0 = (x1cx2 + y1_0cy*y1_0cy - r2).abs();
    let check1_1 = (x1cx2 + y1_1cy*y1_1cy - r2).abs();
    let y1 = if check1_0 < check1_1 { y1_0 } else { y1_1 };
    println!("checks1: {:?}, {:?}", check1_0, check1_1);

    return SMatrix::from([ x0, y0, x1, y1 ]);
}

fn main() {
    let (cx, cy, r) = (1., 1., 2.);
    let (value, grad) = jacobian(|v| f(v[0], v[1], v[2]), SMatrix::from([cx, cy, r]));
    println!("{value} {grad}");
}
