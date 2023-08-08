use ag::ndarray::arr0;
use autograd as ag;
use ag::{tensor_ops::*, Tensor, VariableEnvironment, variable::GetVariableTensor};

fn main() {
    let mut env = VariableEnvironment::new();
    let cx = env.slot().set(arr0(1.));
    let cy = env.slot().set(arr0(1.));
    let r = env.slot().set(arr0(1.));
    env.run(|g| {
        let cx = g.variable(cx);
        let cy = g.variable(cy);
        let r = g.variable(r);

        let cy2 = square(cy);
        let C = square(cx) + cy2;
        let D = C - square(r) + 1.;
        let a = 4. * C;
        let b: Tensor<'_, f64> = -4. * cx * D;
        let c = square(D) - 4. * cy2;

        let d: Tensor<'_, f64> = square(b) - 4.*a*c;
        let dsqrt = sqrt(d);
        let rhs = dsqrt / 2. / a;
        let lhs = neg(b) / 2. / a;
        let x0: Tensor<f64> = lhs - rhs;
        let y0 = sqrt(1. - square(x0));
        let x1 = lhs + rhs;
        let y1sq = 1. - square(x1);
        let y1 = sqrt(y1sq);

        let eval = |&y, name: &str| {
            let d = &grad::<'_, f64, Tensor<f64>, Tensor<f64>>(&[y], &[cx, cy, r]);
            let val = y.eval(g).unwrap();
            let ran =
                g
                .evaluator()
                .push(d[0]).feed(cx, ag::ndarray::arr0(1.).view())
                .push(d[1]).feed(cy, ag::ndarray::arr0(1.).view())
                .push(d[2]).feed( r, ag::ndarray::arr0(1.).view())
                .run();
            println!("{}: {}", name, val);
            println!("\tcx {:?}", &ran[0].as_ref().unwrap());
            println!("\tcy {:?}", &ran[1].as_ref().unwrap());
            println!("\tr  {:?}", &ran[2].as_ref().unwrap());
        };

        eval(&x0, "x0");
        eval(&y0, "y0");
        eval(&x1, "x1");
        eval(&y1sq, "y1sq");
        eval(&y1, "y1");
    });
}
