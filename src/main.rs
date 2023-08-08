use autograd as ag;
use ag::{tensor_ops::*, Tensor, Float};

fn main() {
    ag::run(|ctx: &mut ag::Context<_>| {
        // let x = ctx.placeholder("x", &[]);
        // let y = ctx.placeholder("y", &[]);
        // let z = 2.*x*x + 3.*y + 1.;

        // dz/dy
        // let gy = &grad(&[z], &[y])[0];
        // println!("gy: {:?}", gy.eval(ctx));

        // let r = ctx.placeholder("r", &[]);
        // let A = r * r;

        // dA/dr
        // let dAdr = &grad(&[A], &[r])[0];
        // let feed = ag::ndarray::arr0(3.);
        // println!("dAdr: {:?}", ctx.evaluator().push(dAdr).feed(r, feed.view()).run()[0]);

        // dz/dx (requires to fill the placeholder `x`)
        // let gx = &grad(&[z], &[x])[0];
        // let feed = ag::ndarray::arr0(2.);
        // println!("gx: {:?}", ctx.evaluator().push(gx).feed(x, feed.view()).run()[0]);  // => Ok(8.)

        // // ddz/dx (differentiates `z` again)
        // let ggx = &grad(&[gx], &[x])[0];
        // println!("ggx: {:?}", ggx.eval(ctx));  // => Ok(4.)

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
        // let dsqrt = d.sqrt();
        let rhs = dsqrt / 2. / a;
        let lhs = neg(b) / 2. / a;
        let x0: Tensor<f64> = lhs - rhs;
        let y0 = sqrt(1. - x0*x0);
        let x1 = lhs + rhs;
        let y1 = sqrt(1. - x1*x1);
        // fn eval(y: &Tensor<'_, f64>, x: impl Placeholder, v: f64, ctx: &mut ag::Context<f64>) -> Result<f64, ag::EvalError> {
        //     let feed = ag::ndarray::arr0(v);
        //     return ctx.evaluator().push(y).feed(x, feed.view()).run()[0];
        // }

        // println!("x0 eval: {:?}", x0.eval(ctx));
        // eval(&x0, cx, 1., ctx);

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
        // fn eval<'graph, F: Float, A>(y: A)
        // where
        //     A: AsRef<Tensor<'graph, F>>,

        // let d0 = &grad(&[x0], &[cx, cy, r]);
        // let ran =
        //     ctx
        //     .evaluator()
        //     .push(d0[0])
        //     .feed(cx, ag::ndarray::arr0(1.).view())
        //     .push(d0[1])
        //     .feed(cy, ag::ndarray::arr0(1.).view())
        //     .push(d0[2])
        //     .feed(r, ag::ndarray::arr0(1.).view())
        //     .run();
        // println!("dx0/cx {:?}", &ran[0].as_ref().unwrap());
        // println!("dx0/cy {:?}", &ran[1].as_ref().unwrap());
        // println!("dx0/r  {:?}", &ran[2].as_ref().unwrap());
        // println!("ran {:?}", ran);

        // let d1 = &grad(&[x1], &[cx, cy, r]);
        // let ran =
        //     ctx
        //     .evaluator()
        //     .push(d1[0])
        //     .feed(cx, ag::ndarray::arr0(1.).view())
        //     .push(d1[1])
        //     .feed(cy, ag::ndarray::arr0(1.).view())
        //     .push(d1[2])
        //     .feed(r, ag::ndarray::arr0(1.).view())
        //     .run();
        // println!("dx1/cx {:?}", &ran[0].as_ref().unwrap());
        // println!("dx1/cy {:?}", &ran[1].as_ref().unwrap());
        // println!("dx1/r  {:?}", &ran[2].as_ref().unwrap());

        // let val = &ran[0].as_ref().unwrap();
        // println!("dcdx: {} ({:?})", val, val);

        // match val {
        //     Ok(val) => {
        //         println!("dcdx: {:?}", val);
        //         println!("val: {}", val);
        //     },
        //     Err(err) =>
        //     println!("dcdx: err {:?}", err),
        // }

        // let val = {
        //     let feed = ag::ndarray::arr0(2.);
        //     let pushed = ctx.evaluator().push(dcdx);
        //     let fed = pushed.feed(cx, feed.view());
        //     let ran = fed.run();
        //     &ran[0]
        // };

        // let if Ok(val) = val {
        //     println!("dCdx: {:?}", val);
        // } else {
        //     println!("dCdx: err");
        // }

        // println!("dCdx: {:?}",);

        // let dCdy = &grad(&[C], &[cy])[0];
        // print!("dCdy: {:?}", dCdy.eval(ctx));

        // dz/dy
        // let gy = &grad(&[z], &[y])[0];
        // println!("{:?}", gy.eval(ctx));   // => Ok(3.)

        // // dz/dx (requires to fill the placeholder `x`)
        // let gx = &grad(&[z], &[x])[0];
        // let feed = ag::ndarray::arr0(2.);
        // println!("{:?}", ctx.evaluator().push(gx).feed(x, feed.view()).run()[0]);  // => Ok(8.)

        // // ddz/dx (differentiates `z` again)
        // let ggx = &grad(&[gx], &[x])[0];
        // println!("{:?}", ggx.eval(ctx));  // => Ok(4.)
     });
}
