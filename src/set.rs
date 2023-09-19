use std::{rc::Rc, cell::RefCell};

use crate::{shape::Shape, component::C, zero::Zero};

pub type S<D> = Rc<RefCell<Set<D>>>;

#[derive(Clone, Debug)]
pub struct Set<D> {
    pub idx: usize,
    pub children: Vec<C<D>>,
    pub shape: Shape<D>,
}

impl<D> Set<D> {
    pub fn new(idx: usize, shape: Shape<D>) -> S<D> {
        Rc::new(RefCell::new(Set {
            idx,
            children: vec![],
            shape,
        }))
    }
}
impl<D: Zero> Set<D> {
    pub fn zero(&self) -> D {
        self.shape.zero()
    }
}