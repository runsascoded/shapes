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

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use circle::Split;
use diagram::Diagram;
use dual::Dual;
use log::info;
use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use web_sys::console;
use crate::circle::Circle;
use crate::diagram::{Error, Targets};
use crate::model::Model;
use crate::r2::R2;
use tsify::Tsify;

#[derive(Tsify, Serialize, Deserialize)]
pub struct JsDual {
    v: f64,
    d: Vec<f64>,
}

impl From<&Dual> for JsDual {
    fn from(d: &Dual) -> Self {
        JsDual {
            v: d.v().clone(),
            d: d.d().clone(),
        }
    }
}

impl From<Rc<RefCell<Circle<Dual>>>> for Circle<JsDual> {
    fn from(d: Rc<RefCell<Circle<Dual>>>) -> Self {
        let d = d.borrow();
        Circle {
            idx: d.idx,
            c: R2 { x: JsDual::from(&d.c.x), y: JsDual::from(&d.c.y) },
            r: JsDual::from(&d.r),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize)]
pub struct JsPoint<T> {
    x: T,
    y: T,
}
#[derive(Tsify, Serialize, Deserialize)]
pub struct JsCircle {
    idx: usize,
    c: JsPoint<JsDual>,
    r: JsDual,
}

#[derive(Tsify, Serialize, Deserialize)]
pub struct JsErr {
    pub key: String,
    pub actual_area: Option<JsDual>,
    pub total_area: JsDual,
    pub actual_frac: JsDual,
    pub target_area: f64,
    pub total_target_area: f64,
    pub target_frac: f64,
    pub error: JsDual,
}

impl From<Error> for JsErr {
    fn from(e: Error) -> Self {
        JsErr {
            key: e.key,
            actual_area: e.actual_area.map(|a| (&a).into()),
            total_area: (&e.total_area).into(),
            actual_frac: (&e.actual_frac).into(),
            target_area: e.target_area,
            total_target_area: e.total_target_area,
            target_frac: e.target_frac,
            error: (&e.error).into(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize)]
pub struct JsDiagram {
    shapes: Vec<Circle<f64>>,
    duals: Vec<Circle<JsDual>>,
    error: JsDual,
    errors: HashMap<String, JsErr>,
}

impl From<Rc<RefCell<Diagram>>> for JsDiagram {
    fn from(d: Rc<RefCell<Diagram>>) -> Self {
        JsDiagram {
            shapes: d.borrow().shapes.shapes.clone(),
            duals: d.borrow().shapes.duals.iter().map(|d| From::from(d.clone())).collect(),
            error: (&d.borrow().error).into(),
            errors: d.borrow().errors.iter().map(|(k, v)| (k.clone(), v.clone().into())).collect(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize)]
pub struct JsModel {
    pub steps: Vec<JsDiagram>,
    pub repeat_idx: Option<usize>,
    pub min_idx: usize,
    pub min_step: JsDiagram,
    pub error: JsDual,
}

impl From<Model> for JsModel {
    fn from(m: Model) -> Self {
        JsModel {
            steps: m.steps.iter().map(|d| { From::from(d.clone()) }).collect(),
            repeat_idx: m.repeat_idx,
            min_idx: m.min_idx,
            min_step: m.min_step.clone().into(),
            error: (&m.min_step.borrow().error).into(),
        }
    }
}

#[wasm_bindgen]
pub fn diagram(circles: &JsValue, targets: &JsValue) -> JsValue {
    log::set_logger(&DEFAULT_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Info);
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
            [c.c.x.d.clone(), c.c.y.d.clone(), c.r.d.clone(), ],
        )
    }).collect();
    console::log_1(&"made splits".into());

    let targets: Vec<(String, f64)> = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
    console::log_1(&"target tuples".into());
    let targets: Targets = targets.into_iter().collect();
    console::log_1(&"targets".into());
    let diagram = Diagram::new(circles, targets, None);
    let js_diagram: JsDiagram = From::from(Rc::new(RefCell::new(diagram)));
    serde_wasm_bindgen::to_value(&js_diagram).unwrap()
}

#[wasm_bindgen]
pub fn step(circles: &JsValue, targets: &JsValue, step_size: f64) -> JsValue {
    log::set_logger(&DEFAULT_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Info);
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

    let targets: Vec<(String, f64)> = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
    console::log_1(&"target tuples".into());
    let targets: Targets = targets.into_iter().collect();
    console::log_1(&"targets".into());
    let mut diagram = Diagram::new(circles, targets, None);
    diagram.step(step_size);
    let js_diagram: JsDiagram = From::from(Rc::new(RefCell::new(diagram)));
    // let shapes = diagram.shapes.shapes;
    // let errors = diagram.errors;
    // let error = diagram.error;
    // let model = Model::new(circles, targets, step_size, max_steps);
    // console::log_1(&"model".into());
    // let js_model: JsModel = model.into();
    // console::log_1(&"js_model".into());
    serde_wasm_bindgen::to_value(&js_diagram).unwrap()
}

#[wasm_bindgen]
pub fn fit(circles: &JsValue, targets: &JsValue, step_size: f64, max_steps: usize) -> JsValue {
    log::set_logger(&DEFAULT_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Info);
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

    let targets: Vec<(String, f64)> = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
    console::log_1(&"target tuples".into());
    let targets: Targets = targets.into_iter().collect();
    console::log_1(&"targets".into());
    let model = Model::new(circles, targets, step_size, max_steps);
    console::log_1(&"model".into());
    let js_model: JsModel = model.into();
    console::log_1(&"js_model".into());
    serde_wasm_bindgen::to_value(&js_model).unwrap()
}
