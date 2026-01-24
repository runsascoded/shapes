use tsify::declare;

/// Manually export TS bindings for crate::dual::Dual (rather than deal with its DualDVec64 member)
#[declare]
#[allow(dead_code)]
struct Dual {
    pub v: f64,
    pub d: Vec<f64>,
}
