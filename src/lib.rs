#[macro_use]
extern crate approx;

mod circle;
mod dual;
mod edge;
mod intersection;
mod r2;
mod region;
mod shapes;

use dual::Dual;
use wasm_bindgen::prelude::*;
use crate::circle::Circle;
use crate::r2::R2;

#[wasm_bindgen]
extern {
    pub fn alert(s: &str);
}

#[wasm_bindgen]
pub fn circle(cx: f64, cy: f64, r: f64) -> JsValue {
    let circle = Circle {
        idx: 0,
        c: R2 { x: cx, y: cy },
        r,
    };
    serde_wasm_bindgen::to_value(&circle).unwrap()
}

// #[wasm_bindgen]
// pub fn unit_intersection_duals(val: JsValue) -> JsValue {
//     let circle: Circle<f64> = serde_wasm_bindgen::from_value(val).unwrap();
//     let duals = circle.unit_intersection_duals();
//     serde_wasm_bindgen::to_value(&duals).unwrap()
// }
