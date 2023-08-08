use autograd as ag;
use ag::{tensor_ops::*, Tensor};

fn main() {
    ag::run(|ctx: &mut ag::Context<_>| {
        let cx: Tensor<f64> = ctx.placeholder("cx", &[]);
        let cy: Tensor<f64> = ctx.placeholder("cy", &[]);
        let r: Tensor<f64> = ctx.placeholder("r", &[]);
        let C: Tensor<f64> = cx*cx + cy*cy;
        let D = C - r*r + 1.;
        let a = 4. * C;
        let b: Tensor<'_, f64> = -4. * cx * D;
        let c = D*D - 4. * cy*cy;

        let d: Tensor<'_, f64> = b*b - 4.*a*c;
        let dsqrt = sqrt(d);
        let rhs = dsqrt / 2. / a;
        let lhs = neg(b) / 2. / a;
        let x0: Tensor<f64> = lhs - rhs;
        let y0 = sqrt(1. - x0*x0);
        let x1 = lhs + rhs;
        let y1 = sqrt(1. - x1*x1);

        let eval = |&y| {
            let d0 = &grad::<'_, f64, Tensor<f64>, Tensor<f64>>(&[y], &[cx, cy, r]);
            let ran =
                ctx
                .evaluator()
                .push(d0[0])
                .feed(cx, ag::ndarray::arr0(1.).view())
                .push(d0[1])
                .feed(cy, ag::ndarray::arr0(1.).view())
                .push(d0[2])
                .feed(r, ag::ndarray::arr0(1.).view())
                .run();
            println!("\tcx {:?}", &ran[0].as_ref().unwrap());
            println!("\tcy {:?}", &ran[1].as_ref().unwrap());
            println!("\tr  {:?}", &ran[2].as_ref().unwrap());
        };

        println!("dx0");
        eval(&x0);
        println!("dy0");
        eval(&y0);
        println!("dx1");
        eval(&x1);
        println!("dy1");
        eval(&y1);
     });
}
