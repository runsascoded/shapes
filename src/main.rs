use autograd as ag;
use ag::{tensor_ops::*, Tensor};

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

        let cx: Tensor<'_, f64> = ctx.placeholder("cx", &[]);
        let cy: Tensor<'_, f64> = ctx.placeholder("cy", &[]);
        let r: Tensor<'_, f64> = ctx.placeholder("r", &[]);
        let C: Tensor<'_, f64> = cx*cx + cy+cy;
        // let D = C - r*r + 1;
        // let b2a = cx*D / 2 / C;

        // fn eval(y: &Tensor<'_, f64>, x: impl Placeholder, v: f64, ctx: &mut ag::Context<f64>) -> Result<f64, ag::EvalError> {
        //     let feed = ag::ndarray::arr0(v);
        //     return ctx.evaluator().push(y).feed(x, feed.view()).run()[0];
        // }

        // let z = 2.*x*x + 3.*y + 1.;
        let dcdx = &grad(&[C], &[cx])[0];
        let feed = ag::ndarray::arr0(2.);
        let ran = ctx.evaluator().push(dcdx).feed(cx, feed.view()).run();
        let val = &ran[0];
        // let val = {
        //     let feed = ag::ndarray::arr0(2.);
        //     let pushed = ctx.evaluator().push(dcdx);
        //     let fed = pushed.feed(cx, feed.view());
        //     let ran = fed.run();
        //     &ran[0]
        // };
        match val {
            Ok(val) =>
                println!("dcdx: {:?}", val),
            Err(err) => println!("dcdx: err {:?}", err),
        }
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
