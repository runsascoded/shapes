use std::{rc::Rc, cell::RefCell, collections::{BTreeSet, BTreeMap}};

use serde::{Serialize, Deserialize};
use tsify::Tsify;

use crate::{shape::Shape, zero::Zero, component::{Component, self}, dual::Dual};

pub type S<D> = Rc<RefCell<Set<D>>>;

/// Wrapper around a [`Shape`] within a [`Scene`]; stores [`component::Key`]s of components that are contained within this [`Shape`].
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
pub struct Set<D> {
    pub idx: usize,
    pub child_component_keys: BTreeSet<component::Key>,
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
            child_component_keys: self.child_component_keys.clone(),
            shape: self.shape.v(),
        }
    }
}
impl<D> Set<D> {
    pub fn new(idx: usize, shape: Shape<D>) -> Self {
        Self {
            idx,
            child_component_keys: BTreeSet::new(),
            shape,
        }
    }
    /// Only store direct descendents in `child_component_keys`.
    pub fn prune_children(&mut self, components: &BTreeMap<component::Key, Component<D>>) {
        self.child_component_keys = self.descendent_depths(components).into_iter().filter(|(_, v)| *v == 1).map(|(k, _)| k).collect();
    }
    pub fn descendent_depths(&self, components: &BTreeMap<component::Key, Component<D>>) -> BTreeMap<component::Key, usize> {
        let mut descendent_depths: BTreeMap<component::Key, usize> = BTreeMap::new();
        for child_component_key in &self.child_component_keys {
            if !descendent_depths.contains_key(child_component_key) {
                descendent_depths.insert(child_component_key.clone(), 1);
            }
            let child_component = &components[child_component_key];
            for child_set in &child_component.sets {
                let cur_depths: BTreeMap<component::Key, usize> =
                    child_set
                    .borrow()
                    .descendent_depths(components)
                    .into_iter()
                    .map(|(k, v)| (k, v + 1))
                    .collect();
                for (k, v) in &cur_depths {
                    if !descendent_depths.contains_key(k) || descendent_depths[k] < *v {
                        descendent_depths.insert(k.clone(), *v);
                    }
                }
            }
        }
        // debug!("Set {}: children {:?}, descendent depths: {:?}", self.idx, self.child_component_keys, descendent_depths);
        descendent_depths
    }
}