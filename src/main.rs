use ag::ndarray::{arr0, self, IxDyn};
use autograd as ag;
use ag::tensor_ops as T;
use ag::{tensor_ops::*, Tensor, VariableEnvironment, variable::GetVariableTensor};

fn main() {
    let mut env = VariableEnvironment::new();
    let cx = env.slot().set(arr0(1.));
    let cy = env.slot().set(arr0(0.1));
    let r = env.slot().set(arr0(1.));
    env.run(|g| {
        let cx = g.variable(cx);
        let cy = g.variable(cy);
        let r = g.variable(r);

        let cx2 = square(cx);
        let cy2 = square(cy);
        let C = cx2 + cy2;
        let r2 = square(r);
        let D = C - r2 + 1.;
        let a = 4. * C;
        let b: Tensor<'_, f64> = -4. * cx * D;
        let c = square(D) - 4. * cy2;

        let d: Tensor<'_, f64> = square(b) - 4.*a*c;
        let dsqrt = sqrt(d);
        let rhs = dsqrt / 2. / a;
        let lhs = neg(b) / 2. / a;

        let ixdyn = IxDyn(&[]);

        let x0: Tensor<f64> = lhs - rhs;
        let y0sq = 1. - square(x0);
        let y0_0 = neg(sqrt(y0sq));
        let y0_1 = sqrt(y0sq);
        let x0cx2 = square(x0 - cx);
        let check0_0 = abs(x0cx2 + square(y0_0 - cy) - r2).eval(g).unwrap()[&ixdyn];
        let check0_1 = abs(x0cx2 + square(y0_1 - cy) - r2).eval(g).unwrap()[&ixdyn];
        let y0 = if check0_0 < check0_1 { y0_0 } else { y0_1 };
        println!("checks0: {:?}, {:?}", check0_0, check0_1);

        let x1 = lhs + rhs;
        let y1sq = 1. - square(x1);
        let y1_0 = neg(sqrt(y1sq));
        let y1_1 = sqrt(y1sq);
        let x1cx2 = square(x1 - cx);
        let check1_0 = abs(x1cx2 + square(y1_0 - cy) - r2).eval(g).unwrap()[&ixdyn];
        let check1_1 = abs(x1cx2 + square(y1_1 - cy) - r2).eval(g).unwrap()[&ixdyn];
        let y1 = if check1_0 < check1_1 { y1_0 } else { y1_1 };
        println!("checks1: {:?}, {:?}", check1_0, check1_1);

        let eval = |&y, name: &str| {
            let d = &grad::<'_, f64, Tensor<f64>, Tensor<f64>>(&[y], &[cx, cy, r]);
            let val = y.eval(g).unwrap();
            let ran =
                g
                .evaluator()
                .push(d[0]).feed(cx, arr0(1.).view())
                .push(d[1]).feed(cy, arr0(1.).view())
                .push(d[2]).feed( r, arr0(1.).view())
                .run();
            println!("{}: {}", name, val);
            println!("\tcx {}", &ran[0].as_ref().unwrap()[&ixdyn]);
            println!("\tcy {}", &ran[1].as_ref().unwrap()[&ixdyn]);
            println!("\tr  {}", &ran[2].as_ref().unwrap()[&ixdyn]);
        };

        eval(&x0, "x0");
        eval(&y0, "y0");
        eval(&x1, "x1");
        eval(&y1, "y1");
    });
}
