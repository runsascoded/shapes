#[macro_use]
extern crate approx;
extern crate console_error_panic_hook;

mod areas;
mod circle;
mod diagram;
mod deg;
mod dual;
mod edge;
mod intersection;
mod math;
mod model;
mod r2;
mod region;
mod shapes;
mod zero;

use circle::Split;
use dual::Dual;
use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use web_sys::console;
use crate::circle::Circle;
use crate::diagram::Targets;
use crate::model::Model;
use crate::r2::R2;

#[derive(Serialize, Deserialize)]
struct JsDual {
    v: f64,
    d: Vec<f64>,
}

impl From<&Dual> for JsDual {
    fn from(d: &Dual) -> Self {
        JsDual {
            v: d.v(),
            d: d.d(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct JsModel {
    shapes: Vec<Circle<f64>>,
    duals: Vec<Circle<JsDual>>,
    error: f64,
}

impl From<Model> for JsModel {
    fn from(m: Model) -> Self {
        let min_step = m.min_step.borrow();
        let duals: Vec<Circle<JsDual>> = min_step.shapes.duals.iter().map(|c| {
            Circle {
                idx: c.borrow().idx,
                c: R2 { x: JsDual::from(&c.borrow().c.x), y: JsDual::from(&c.borrow().c.y) },
                r: JsDual::from(&c.borrow().r),
            }
        }).collect();
        JsModel {
            shapes: duals.iter().map(|c| {
                Circle {
                    idx: c.idx,
                    c: R2 { x: c.c.x.v, y: c.c.y.v },
                    r: c.r.v,
                }
            }).collect(),
            duals,
            error: min_step.error.re,
        }
    }
}

#[wasm_bindgen]
pub fn circle(cx: f64, cy: f64, r: f64) -> JsValue {
    console::log_1(&"web-sys circle".into());
    let circle = Circle {
        idx: 0,
        c: R2 { x: cx, y: cy },
        r,
    };
    serde_wasm_bindgen::to_value(&circle).unwrap()
}

#[wasm_bindgen]
pub fn fit(circles: &JsValue, targets: &JsValue, step_size: f64, max_steps: usize) -> JsValue {
    console_error_panic_hook::set_once();
    console::log_2(&"web-sys fit".into(), circles);
    let circles: Vec<Circle<JsDual>> = serde_wasm_bindgen::from_value(circles.clone()).unwrap();
    let circles: Vec<Split> = circles.iter().map(|c| {
        (
            Circle {
                idx: c.idx,
                c: R2 { x: c.c.x.v, y: c.c.y.v },
                r: c.r.v,
            },
            [ c.c.x.d.clone(), c.c.y.d.clone(), c.r.d.clone(), ],
        )
    }).collect();
    console::log_1(&"made splits".into());

    //     Circle {
    //         idx: c.idx,
    //         c: R2 { x: Dual::new(c.c.x.v, c.c.x.d), y: Dual::new(c.c.y.v, c.c.y.d) },
    //         r: Dual::new(c.r.v, c.r.d),
    //     }
    // });
    let targets: Vec<(String, f64)> = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
    console::log_1(&"target tuples".into());
    let targets: Targets = targets.into_iter().collect();
    console::log_1(&"targets".into());
    let model = Model::new(circles, targets, step_size, max_steps);
    console::log_1(&"model".into());
    let js_model: JsModel = model.into();
    // let min_step = &model.min_step.borrow();
    // let error = min_step.error;
    // let shapes = min_step.shapes.shapes;
    console::log_1(&"js_model".into());
    serde_wasm_bindgen::to_value(&js_model).unwrap()
}
