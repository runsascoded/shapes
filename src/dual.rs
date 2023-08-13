use nalgebra::{Dim, DefaultAllocator, allocator::Allocator, U3, U1, Const};
use num_dual::{DualVec, DualVec64};
use serde::{Deserialize, Serialize};



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dual {
    pub v: f64,
    pub d: Vec<f64>
}

impl From<DualVec64<Const<3>>> for Dual
where
    DefaultAllocator: Allocator<f64, Const<3>>
{
    fn from(dv: DualVec64<Const<3>>) -> Self {
        Dual {
            v: dv.re,
            d: dv.eps.unwrap_generic(U3, U1).as_slice().to_vec()
        }
    }
}