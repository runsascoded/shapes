use std::{rc::Rc, cell::RefCell, collections::{BTreeSet, BTreeMap}};

use serde::{Serialize, Deserialize};
use tsify::Tsify;

use crate::{shape::Shape, zero::Zero, component::Component, dual::Dual};

pub type S<D> = Rc<RefCell<Set<D>>>;

#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
pub struct Set<D> {
    pub idx: usize,
    pub children: BTreeSet<usize>,
    pub shape: Shape<D>,
}

impl<D: Zero> Set<D> {
    pub fn zero(&self) -> D {
        self.shape.zero()
    }
}

impl Set<Dual> {
    pub fn v(&self) -> Set<f64> {
        Set {
            idx: self.idx,
            children: self.children.clone(),
            shape: self.shape.v(),
        }
    }
}
impl<D> Set<D> {
    pub fn new(idx: usize, shape: Shape<D>) -> Self {
        Self {
            idx,
            children: BTreeSet::new(),
            shape,
        }
    }
    pub fn prune_children(&mut self, components: &Vec<Component<D>>) {
        self.children = self.descendent_depths(components).into_iter().filter(|(_, v)| *v == 1).map(|(k, _)| k).collect();
    }
    pub fn descendent_depths(&self, components: &Vec<Component<D>>) -> BTreeMap<usize, usize> {
        let mut descendent_depths: BTreeMap<usize, usize> = BTreeMap::new();
        for child_component_idx in &self.children {
            if !descendent_depths.contains_key(child_component_idx) {
                descendent_depths.insert(*child_component_idx, 1);
            }
            let child_component = &components[*child_component_idx];
            for child_set in &child_component.sets {
                let cur_depths: BTreeMap<usize, usize> = child_set.borrow().descendent_depths(components).into_iter().map(|(k, v)| (k, v + 1)).collect();
                for (k, v) in &cur_depths {
                    if !descendent_depths.contains_key(k) || descendent_depths[k] < *v {
                        descendent_depths.insert(*k, *v);
                    }
                }
            }
        }
        descendent_depths
    }
}