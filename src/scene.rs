use core::f64;
use std::{cell::RefCell, rc::Rc, collections::{BTreeSet, BTreeMap}, ops::{Neg, Add, Sub, Mul, Div}};

use log::{debug, info, error};
use ordered_float::OrderedFloat;

use crate::{node::{N, Node}, contains::{Contains, ShapeContainsPoint}, distance::Distance, error::SceneError, region::RegionArg, set::S, shape::{Shape, AreaArg}, theta_points::ThetaPoints, intersect::{Intersect, IntersectShapesArg}, r2::R2, transform::{CanTransform, HasProjection, CanProject}, dual::Dual, to::To, math::deg::Deg, fmt::Fmt, component::{Component, self}, set::Set};

/// Collection of [`Shape`]s (wrapped in [`Set`]s), and segmented into connected [`Component`]s.
#[derive(Clone, Debug)]
pub struct Scene<D> {
    pub sets: Vec<S<D>>,
    pub components: Vec<Component<D>>,
}

pub trait SceneD
: IntersectShapesArg
+ AreaArg
+ Add<Output = Self>
+ Mul<f64, Output = Self>
+ Div<f64, Output = Self>
+ Deg
+ Fmt
+ RegionArg
+ ShapeContainsPoint
+ component::ComponentArg
{}
impl SceneD for f64 {}
impl SceneD for Dual {}
pub trait SceneR2<D>
: Neg<Output = Self>
// TODO: can't get this to derive for R2<Dual>
// + Mul<f64, Output = Self>
// + Div<f64, Output = Self>
+ CanTransform<D, Output = Self>
+ To<R2<f64>>
// ShapeContainsPoint
+ CanProject<D, Output = R2<D>>
{}
impl SceneR2<f64> for R2<f64> {}
impl SceneR2<Dual> for R2<Dual> {}
pub trait SceneFloat<D>
: Add<D, Output = D>
+ Sub<D, Output = D>
+ Mul<D, Output = D>
+ Div<D, Output = D>
{}
impl SceneFloat<f64> for f64 {}
impl SceneFloat<Dual> for f64 {}

impl<D: SceneD> Scene<D>
where
    R2<D>: Into<R2<f64>> + SceneR2<D>,
    Shape<f64>: From<Shape<D>>,
    Shape<D>: CanTransform<D, Output = Shape<D>> + HasProjection<D>,
    f64: SceneFloat<D>,
{
    pub fn new(shapes: Vec<Shape<D>>) -> Result<Scene<D>, SceneError> {
        let num_shapes = shapes.len();
        let sets = shapes.into_iter().enumerate().map(|(idx, shape)| Set::new(idx, shape)).collect::<Vec<_>>();
        let mut set_ptrs: Vec<S<D>> = sets.into_iter().map(|s| Rc::new(RefCell::new(s))).collect();
        let mut nodes: Vec<N<D>> = Vec::new();
        let merge_threshold = 1e-7;
        let zero = set_ptrs[0].borrow().zero();

        let mut is_directly_connected: Vec<Vec<bool>> = Vec::new();
        // Intersect all shapes, pair-wise
        for (idx, set_ptr) in set_ptrs.iter().enumerate() {
            let mut directly_connected: Vec<bool> = Vec::new();
            if idx > 0 {
                for jdx in 0..idx {
                    directly_connected.push(is_directly_connected[jdx][idx]);
                }
            }
            let shape0 = &set_ptr.borrow().shape;
            directly_connected.push(true);
            for jdx in (idx + 1)..num_shapes {
                let set1_ref = set_ptrs[jdx].borrow();
                let shape1 = &set1_ref.shape;
                let mut intersections = shape0.intersect(shape1);
                let mut i = 0;
                loop {
                    if i >= intersections.len() { break }
                    let cur = &intersections[i];
                    if i + 1 < intersections.len() && cur.distance(&intersections[i + 1]).into() < merge_threshold {
                        info!("Skipping apparent tangent point: {} == {}", cur, intersections[i + 1]);
                        intersections.remove(i);
                        intersections.remove(i);
                    } else {
                        i += 1;
                    }
                }
                if intersections.is_empty() {
                    directly_connected.push(false);
                } else {
                    directly_connected.push(true);
                }
                for i in intersections {
                    let mut merged = false;
                    let theta0 = shape0.theta(&i);
                    let theta1 = shape1.theta(&i);
                    for node in &nodes {
                        let d = node.borrow().p.distance(&i);
                        if d.into() < merge_threshold {
                            // This intersection is close enough to an existing node; merge them
                            let mut node = node.borrow_mut();
                            node.merge(i.clone(), idx, &theta0, jdx, &theta1);
                            info!("Merged: {} into {}", i, node);
                            merged = true;
                            break;
                        }
                    }
                    if merged {
                        continue;
                    }
                    let node = Node {
                        idx: nodes.len(),
                        p: i,
                        n: 1,
                        shape_thetas: vec![(idx, theta0), (jdx, theta1)].into_iter().collect(),
                        edges: Vec::new(),
                    };
                    let n = Rc::new(RefCell::new(node));
                    nodes.push(n.clone());
                }
            }
            is_directly_connected.push(directly_connected);
        }

        debug!("{} nodes", nodes.len());
        for (idx, node) in nodes.iter().enumerate() {
            debug!("  Node {}: {}", idx, node.borrow());
        }
        debug!("");

        let mut nodes_by_shape: Vec<Vec<N<D>>> = Vec::new();
        for _idx in 0..num_shapes {
            nodes_by_shape.push(Vec::new());
        }
        // Compute connected components (shape -> shape -> bool)
        let mut is_connected: Vec<Vec<bool>> = is_directly_connected.clone();
        for i0 in 0..num_shapes {
            for i1 in 0..num_shapes {
                if is_connected[i0][i1] {
                    for i2 in 0..num_shapes {
                        if is_connected[i0][i2] && !is_connected[i1][i2] {
                            for i3 in 0..num_shapes {
                                if is_connected[i1][i3] && !is_connected[i2][i3] {
                                    is_connected[i2][i3] = true;
                                    is_connected[i3][i2] = true;
                                }
                            };
                        }
                    };
                }
            }
        }
        debug!("is_directly_connected: {:?}", is_directly_connected);
        debug!("is_connected: {:?}", is_connected);

        for node in &nodes {
            node.borrow().shape_thetas.keys().for_each(|i0| {
                nodes_by_shape[*i0].push(node.clone());
            });
        }

        // shape_idx -> shape_idxs
        let mut unconnected_containers: Vec<BTreeSet<usize>> = Vec::new();
        for (idx, set_ptr) in set_ptrs.iter().enumerate() {
            let shape = &set_ptr.borrow().shape;
            let mut containers: BTreeSet<usize> = BTreeSet::new();
            for (jdx, container) in set_ptrs.iter().enumerate() {
                if !is_connected[idx][jdx] &&
                    // This check can false-positive if the shapes are tangent at theta == 0, so we check center-containment below as well
                    container.borrow().shape.contains(&shape.point(zero.clone())) &&
                    // Concentric shapes contain each others' centers, so this check alone is insufficient (needs the above check as well)
                    container.borrow().shape.contains(&shape.center()) {
                    containers.insert(jdx);
                }
            }
            unconnected_containers.push(containers);
        }

        // Sort each circle's nodes in order of where they appear on the circle (from -PI to PI)
        for (idx, nodes) in nodes_by_shape.iter_mut().enumerate() {
            nodes.sort_by_cached_key(|n| OrderedFloat(n.borrow().theta(idx).into()))
        }

        let components_idxs: Vec<Vec<usize>> = {
            let mut assigned_idxs: BTreeSet<usize> = BTreeSet::new();
            let mut components_idxs: Vec<Vec<usize>> = Vec::new();
            for shape_idx in 0..num_shapes {
                if assigned_idxs.contains(&shape_idx) {
                    continue
                }
                let connection_idxs = is_connected[shape_idx].clone().into_iter().enumerate().filter(|(_, c)| *c).map(|(i, _)| i).collect::<Vec<usize>>();
                components_idxs.push(connection_idxs.clone());
                for idx in connection_idxs {
                    assigned_idxs.insert(idx);
                }
            }
            components_idxs
        };
        debug!("Making components: {:?}", components_idxs);
        let mut components: Vec<Component<D>> =
            components_idxs
            .into_iter()
            .map(|component_idxs| Component::new(component_idxs, &unconnected_containers, &nodes_by_shape, &set_ptrs, num_shapes))
            .collect();

        let components_map = components.iter().map(|c| (c.key.clone(), c.clone())).collect::<BTreeMap<_, _>>();

        let component_ptrs = components.iter().map(|c| Rc::new(RefCell::new(c.clone()))).collect::<Vec<_>>();
        for component in components.iter() {
            for set_idx in &component.container_set_idxs {
                // debug!("Set {} contains component {}", set_idx, component.key);
                set_ptrs[*set_idx].borrow_mut().child_component_keys.insert(component.key.clone());
            }
        }
        for set in &mut set_ptrs {
            set.borrow_mut().prune_children(&components_map);
            // debug!("Set {}, child components: {}", set.borrow().idx, set.borrow().child_component_keys.iter().map(|k| k.to_string()).collect::<Vec<_>>().join(", "));
        }

        for component_ptr in component_ptrs.iter() {
            let component = component_ptr.borrow();
            match component.verify_areas(0.001) {
                Ok(_) => {}
                Err(err) => {
                    error!("Component {} failed area verification: {}", component.key, err);
                    // panic!("Component {} failed area verification: {}", component.key, err);
                }
            }
            let key = &component.key;
            let p: R2<f64> = component.sets[0].borrow().shape.c().into();
            // debug!("{}: {} at {}", component_idx, key, p);
            for container_component in &mut components {
                // dbg!(children.clone());
                if container_component.contains(key) {
                    container_component.child_component_keys.insert(key.clone());
                    // debug!("  {} contains {}, checking {} regions", container_component.idx, component_idx, container_component.regions.len());
                    let container_shapes: BTreeMap<usize, Shape<f64>> = container_component.sets.iter().map(|s| {
                        let idx = s.borrow().idx;
                        let s = s.borrow().shape.clone().into();
                        (idx, s)
                    }).collect();
                    let mut container_regions: Vec<_> = container_component.regions.iter_mut().filter(|r| {
                        // let contains = r.contains(&p);
                        // debug!("  {} contains {}: {}", r.key, p, contains);
                        // contains
                        r.contains(&p, &container_shapes)
                    }).collect();
                    if container_regions.len() != 1 {
                        return Err(SceneError::ContainerRegionCount {
                            component_key: container_component.key.clone(),
                            child_key: component_ptr.borrow().key.clone(),
                            count: container_regions.len(),
                            regions: container_regions.iter().map(|r| r.key.clone()).collect(),
                        });
                    }
                    container_regions[0].child_components.push(component_ptr.clone());
                }
            }
        }
        let component_depths_map = Scene::compute_component_depths(&mut components)?;
        components.sort_by_cached_key(|c| {
            -*component_depths_map.get(&c.key).expect("component depth should exist after compute_component_depths")
        });
        Ok(Scene { sets: set_ptrs, components, })
    }

    pub fn compute_component_depths(components: &mut Vec<Component<D>>) -> Result<BTreeMap<component::Key, i64>, SceneError> {
        let components_map = components.iter().map(|c| (c.key.clone(), c.clone())).collect::<BTreeMap<_, _>>();
        let mut component_depths_map = BTreeMap::new();
        for component in &mut *components {
            Scene::compute_component_depth(component, &components_map, &mut component_depths_map)?;
        }
        Ok(component_depths_map)
    }

    pub fn compute_component_depth(
        component: &Component<D>,
        components_map: &BTreeMap<component::Key, Component<D>>,
        component_depths_map: &mut BTreeMap<component::Key, i64>,
    ) -> Result<i64, SceneError> {
        match component_depths_map.get(&component.key) {
            Some(depth) => Ok(*depth),
            None => {
                if component.child_component_keys.is_empty() {
                    component_depths_map.insert(component.key.clone(), 0);
                    Ok(0)
                } else {
                    let mut max_depth: Option<i64> = None;
                    for key in &component.child_component_keys {
                        let child_component = components_map.get(key)
                            .ok_or_else(|| SceneError::MissingComponent(key.clone()))?;
                        let child_depth = Scene::compute_component_depth(child_component, components_map, component_depths_map)?;
                        let depth = child_depth + 1;
                        max_depth = Some(max_depth.map_or(depth, |d| d.max(depth)));
                    }
                    let depth = max_depth.ok_or_else(|| SceneError::NoMaxDepth(component.key.clone()))?;
                    component_depths_map.insert(component.key.clone(), depth);
                    Ok(depth)
                }
            }
        }
    }

    pub fn total_area(&self) -> D {
        self.components.iter().filter(|c| c.container_set_idxs.is_empty()).map(|c| c.area()).sum()
    }

    pub fn area(&self, key: &String) -> Option<D> {
        match key.split_once('*') {
            Some((prefix, suffix)) => {
                let k0 = format!("{}-{}", prefix, suffix);
                let k1 = format!("{}{}{}", prefix, prefix.len(), suffix);
                if k0.chars().all(|ch| ch == '-') {
                    self.area(&k1)
                } else {
                    let a0 = self.area(&k0).unwrap_or_else(|| self.zero());
                    let a1 = self.area(&k1).unwrap_or_else(|| self.zero());
                    Some(a0 + a1)
                }
            }
            None => {
                let areas =
                    self
                    .components
                    .iter()
                    .flat_map(|c| c.regions.iter())
                    .filter(|r| &r.key == key)
                    .map(|r| r.area());
                // Manual fold to return None if no matching regions (vs zero)
                areas.fold(None, |acc, area| Some(match acc {
                    None => area,
                    Some(s) => s + area,
                }))
            }
        }
    }

    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    pub fn zero(&self) -> D {
        self.sets[0].borrow().zero()
    }
}

#[cfg(test)]
pub mod tests {
    use std::{collections::BTreeMap, env, f64::consts::PI};

    use itertools::Itertools;
    use log::debug;

    use crate::{math::{deg::Deg, round::round}, dual::Dual, fmt::{DisplayNum, Fmt}, shape::{xyrr, circle, Shapes, xyrrt}, to::To, duals::D};

    use super::*;
    use test_log::test;

    #[test]
    fn test_00_10_01() {
        let c0 = circle(0., 0., 1.);
        let c1 = circle(1., 0., 1.);
        let c2 = circle(0., 1., 1.);
        let inputs = [
            (c0, vec![ D; 3 ]),
            (c1, vec![ D; 3 ]),
            (c2, vec![ D; 3 ]),
        ];
        let shapes = Shapes::from(inputs);
        let scene = Scene::<Dual>::new(shapes.to()).unwrap();
        assert_eq!(scene.components.len(), 1);
        let component = scene.components[0].clone();

        let check = |idx: usize, x: Dual, y: Dual, c0idx: usize, deg0v: i64, deg0d: [i64; 9], c1idx: usize, deg1v: i64, deg1d: [i64; 9]| {
            let n = component.nodes[idx].borrow();
            assert_relative_eq!(n.p.x, x, epsilon = 1e-3);
            assert_relative_eq!(n.p.y, y, epsilon = 1e-3);
            let shape_idxs: Vec<usize> = Vec::from_iter(n.shape_thetas.keys().copied());
            assert_eq!(
                shape_idxs,
                vec![c0idx, c1idx],
            );
            let thetas: Vec<_> = n.shape_thetas.values().map(|t| t.deg()).map(|d| (round(&d.v()), d.d().iter().map(round).collect::<Vec<_>>())).collect();
            assert_eq!(
                thetas,
                vec![
                    (deg0v, deg0d.into()),
                    (deg1v, deg1d.into()),
                ],
            );
        };

        let expected = [
            //    x                                                                     dx         y                                                                    dy c0idx deg0                                       deg0.d c1idx  deg1                                       deg1.d
            ( 0.500, [ 0.500,  0.866,  1.000, 0.500, -0.866, -1.000, 0.000,  0.000,  0.000 ],  0.866, [ 0.289, 0.500,  0.577, -0.289, 0.500,  0.577,  0.000, 0.000,  0.000 ], 0,  60, [  33, -57, -33, -33, 57,  66,   0,   0,   0 ], 1,  120, [ -33, -57, -66,  33, 57,  33,   0,   0,   0 ]),
            ( 0.500, [ 0.500, -0.866,  1.000, 0.500,  0.866, -1.000, 0.000,  0.000,  0.000 ], -0.866, [-0.289, 0.500, -0.577,  0.289, 0.500, -0.577,  0.000, 0.000,  0.000 ], 0, -60, [ -33, -57,  33,  33, 57, -66,   0,   0,   0 ], 1, -120, [  33, -57,  66, -33, 57, -33,   0,   0,   0 ]),
            ( 0.866, [ 0.500,  0.289,  0.577, 0.000,  0.000,  0.000, 0.500, -0.289,  0.577 ],  0.500, [ 0.866, 0.500,  1.000,  0.000, 0.000,  0.000, -0.866, 0.500, -1.000 ], 0,  30, [  57, -33,  33,   0,  0,   0, -57,  33, -66 ], 2,  -30, [  57,  33,  66,   0,  0,   0, -57, -33, -33 ]),
            (-0.866, [ 0.500, -0.289, -0.577, 0.000,  0.000,  0.000, 0.500,  0.289, -0.577 ],  0.500, [-0.866, 0.500,  1.000,  0.000, 0.000,  0.000,  0.866, 0.500, -1.000 ], 0, 150, [  57,  33, -33,   0,  0,   0, -57, -33,  66 ], 2, -150, [  57, -33, -66,   0,  0,   0, -57,  33,  33 ]),
            ( 1.000, [ 0.000,  0.000,  0.000, 0.000,  0.000,  0.000, 1.000,  0.000,  1.000 ],  1.000, [ 0.000, 0.000,  0.000,  0.000, 1.000,  1.000,  0.000, 0.000,  0.000 ], 1,  90, [   0,   0,   0,  57,  0,   0, -57,   0, -57 ], 2,    0, [   0,   0,   0,   0, 57,  57,   0, -57,   0 ]),
            ( 0.000, [ 0.000,  0.000,  0.000, 1.000,  0.000, -1.000, 0.000,  0.000,  0.000 ],  0.000, [ 0.000, 0.000,  0.000,  0.000, 0.000,  0.000,  0.000, 1.000, -1.000 ], 1, 180, [   0,   0,   0,   0, 57,   0,   0, -57,  57 ], 2,  -90, [   0,   0,   0,  57,  0, -57, -57,   0,   0 ]),
        ];
        assert_eq!(component.nodes.len(), expected.len());
        for (idx, (x, dx, y, dy, c0idx, deg0v, deg0d, c1idx, deg1v, deg1d)) in expected.iter().enumerate() {
            let x = Dual::new(*x, dx.iter().copied().collect());
            let y = Dual::new(*y, dy.iter().copied().collect());
            check(idx, x, y, *c0idx, *deg0v, *deg0d, *c1idx, *deg1v, *deg1d);
        }

        assert_eq!(component.edges.len(), 12);
        assert_eq!(component.regions.len(), 7);
        let expected = [
            "01- 0( -60) 2( -30) 1( 180): 0.500 + 0.285 =  0.785, vec![ 1.366, -0.366,  1.571, -0.866, -0.500,  1.047, -0.500,  0.866, -1.047]",
            "-1- 0( -60) 2( -30) 1(  90): -0.000 + -1.785 =  1.785, vec![-1.366,  0.366, -1.571,  1.866, -0.500,  3.665, -0.500,  0.134, -0.524]",
            "-12 0(  30) 1( 120) 2(   0): -0.116 + -0.012 =  0.128, vec![-0.366, -0.366, -0.524, -0.134,  0.500,  0.524,  0.500, -0.134,  0.524]",
            "012 0(  30) 1( 120) 2( -90): 0.250 + 0.193 =  0.443, vec![ 0.366,  0.366,  0.524, -0.866,  0.500,  1.047,  0.500, -0.866,  1.047]",
            "0-2 0(  60) 2(-150) 1( 180): 0.500 + 0.285 =  0.785, vec![-0.366,  1.366,  1.571,  0.866, -0.500, -1.047, -0.500, -0.866,  1.047]",
            "--2 0(  60) 2(-150) 1(  90): -0.000 + -1.785 =  1.785, vec![ 0.366, -1.366, -1.571,  0.134, -0.500, -0.524, -0.500,  1.866,  3.665]",
            "0-- 0( 150) 1(-120) 2( -90): 0.250 + 0.878 =  1.128, vec![-1.366, -1.366,  2.618,  0.866,  0.500, -1.047,  0.500,  0.866, -1.047]",
        ];
        let actual = component.regions.iter().map(|region| {
            let segments = &region.segments;
            let path_str = segments.iter().map(|segment| {
                let start = segment.start();
                let edge = segment.edge.clone();
                let cidx = edge.borrow().set.borrow().idx;
                format!("{}({})", cidx, start.borrow().theta(cidx).v().deg_str())
            }).collect::<Vec<String>>().join(" ");
            format!("{} {}: {:.3} + {:.3} = {}", region.key, path_str, region.polygon_area.v(), region.secant_area.v(), region.area().s(3))
        }).collect::<Vec<String>>();
        actual.iter().zip(expected.iter()).enumerate().for_each(|(idx, (a, b))| assert_eq!(&a, b, "idx: {}, {} != {}", idx, a, b));
    }

    #[test]
    fn test_components() {
        let inputs = [
            (circle(0. , 0., 1.), vec![ D, D, D ]),
            (circle(1. , 0., 1.), vec![ D, D, D ]),
            (circle(0.5, 0., 3.), vec![ D, D, D ]),
            (circle(0. , 3., 1.), vec![ D, D, D ]),
        ];
        let shapes = Shapes::from(inputs);
        let scene = Scene::new(shapes.to_vec()).unwrap();
        assert_eq!(scene.components.len(), 2);
        assert_node_strs(&scene, vec![
              vec![
                "N2( 0.999, vec![ 0.000,  0.000,  0.000,  0.000,  0.000,  0.000,  0.007,  0.042,  0.042,  0.993, -0.042,  0.994],  2.958, vec![ 0.000,  0.000,  0.000,  0.000,  0.000,  0.000,  0.168,  0.993,  1.007, -0.168,  0.007, -0.168]: C2(  80 [   0,    0,    0,    0,    0,    0, 1102,  -46,  138, -1102,   46, -1103]), C3(  -2 [   0,    0,    0,    0,    0,    0,  550, 3263, 3309, -550, -3263, -414]))",
                "N3(-0.932, vec![ 0.000,  0.000,  0.000,  0.000,  0.000,  0.000,  0.175, -0.322, -0.366,  0.825,  0.322, -0.886],  2.636, vec![ 0.000,  0.000,  0.000,  0.000,  0.000,  0.000, -0.448,  0.825,  0.939,  0.448,  0.175, -0.481]: C2( 119 [   0,    0,    0,    0,    0,    0, 1027,  401, -138, -1027, -401, 1103]), C3(-159 [   0,    0,    0,    0,    0,    0, 1579, -2908, -3309, -1579, 2908,  414]))",
              ],
              vec![
                "N0( 0.500, vec![ 0.500,  0.866,  1.000,  0.500, -0.866, -1.000,  0.000,  0.000,  0.000,  0.000,  0.000,  0.000],  0.866, vec![ 0.289,  0.500,  0.577, -0.289,  0.500,  0.577,  0.000,  0.000,  0.000,  0.000,  0.000,  0.000]: C0(  60 [1895, -3283, -1895, -1895, 3283, 3791,    0,    0,    0,    0,    0,    0]), C1( 120 [-1895, -3283, -3791, 1895, 3283, 1895,    0,    0,    0,    0,    0,    0]))",
                "N1( 0.500, vec![ 0.500, -0.866,  1.000,  0.500,  0.866, -1.000,  0.000,  0.000,  0.000,  0.000,  0.000,  0.000], -0.866, vec![-0.289,  0.500, -0.577,  0.289,  0.500, -0.577,  0.000,  0.000,  0.000,  0.000,  0.000,  0.000]: C0( -60 [-1895, -3283, 1895, 1895, 3283, -3791,    0,    0,    0,    0,    0,    0]), C1(-120 [1895, -3283, 3791, -1895, 3283, -1895,    0,    0,    0,    0,    0,    0]))",
              ],
        ]);
    }

    pub fn ellipses4(r: f64) -> [Shape<f64>; 4] {
        ellipses4_select(r, [ 0, 1, 2, 3 ])
    }

    pub fn ellipses4_select<const N: usize>(r: f64, mask: [ usize; N ]) -> [ Shape<f64>; N ] {
        let r2 = r * r;
        let r2sqrt = (1. + r2).sqrt();
        let c0 = 1. / r2sqrt;
        let c1 = r2 * c0;
        let ellipses = [
            xyrr(     c0,      c1, 1., r ),
            xyrr(1. + c0,      c1, 1., r ),
            xyrr(     c1, 1. + c0, r , 1.),
            xyrr(     c1,      c0, r , 1.),
        ];
        mask.map(|i| ellipses[i].clone())
    }

    #[test]
    fn ellipses4_0_2() {
        let shapes = ellipses4_select(2., [0, 2]).to_vec();
        debug!("shapes: {:?}", shapes);
        let scene = Scene::new(shapes).unwrap();
        assert_eq!(scene.components.len(), 1);
        let component = scene.components[0].clone();
        assert_eq!(component.nodes.len(), 2);
        assert_eq!(component.edges.len(), 4);
    }

    #[test]
    fn test_4_ellipses() {
        let scene = Scene::new(ellipses4(2.).into()).unwrap();
        assert_eq!(scene.components.len(), 1);
        let component = scene.components[0].clone();
        assert_eq!(component.nodes.len(), 14);
        assert_eq!(component.edges.len(), 28);
        let mut regions = component.regions;
        regions.sort_by_cached_key(|r| OrderedFloat(r.area()));

        let expected: BTreeMap<&str, f64> = [
            ("01-3", 0.15744583391178407),
            ("0-23", 0.1574458339117857),
            ("0-2-", 0.583046721212134),
            ("-1-3", 0.583046721212141),
            ("0123", 0.6327112800110606),
            ("01--", 0.6911159327218129),
            ("--23", 0.6911159327218173),
            ("0--3", 0.8415779241718198),
            ("012-", 0.9754663505728486),
            ("-123", 0.9754663505728591),
            ("-12-", 1.0010372353827426),
            ("--2-", 1.26689560279433),
            ("-1--", 1.2668956027943392),
            ("---3", 2.2443754306663317),
            ("0---", 2.2443754306663326),
        ].into();
        assert_eq!(regions.len(), expected.len());
        match env::var("GEN_VALS").map(|s| s.parse::<usize>().unwrap()).ok() {
            Some(_) => {
                let mut actual: BTreeMap<&str, f64> = BTreeMap::new();
                for region in &regions {
                    let area = region.area();
                    let key = region.key.as_str();
                    actual.insert(key, area);
                }
                let mut actual: Vec<_> = actual.into_iter().collect();
                actual.sort_by_cached_key(|(_, a)| OrderedFloat(*a));
                for (key, area) in actual {
                    println!("(\"{}\", {}),", key, area);
                }
            }
            None => {
                for region in &regions {
                    let area = region.area();
                    let key = region.key.as_str();
                    assert_relative_eq!(area, *expected.get(key).unwrap(), max_relative = 1e-10);
                }
            }
        }
    }

    #[test]
    fn test_4_circles_lattice_0_1() {
        let shapes = vec![
            xyrr(0., 0., 1., 1.),
            xyrr(0., 1., 1., 1.),
            xyrr(1., 0., 1., 1.),
            xyrr(1., 1., 1., 1.),
        ];
        let scene = Scene::new(shapes).unwrap();
        assert_node_strs(
            &scene,
            vec![vec![
                "N0( 0.866,  0.500: C0(  30), C1( -30))",
                "N1(-0.866,  0.500: C0( 150), C1(-150))",
                "N2( 0.500,  0.866: C0(  60), C2( 120))",
                "N3( 0.500, -0.866: C0( -60), C2(-120))",
                "N4( 0.000,  1.000: C0(  90), C3( 180))",
                "N5( 1.000,  0.000: C0(   0), C3( -90))",
                "N6( 1.000,  1.000: C1(   0), C2(  90))",
                "N7( 0.000,  0.000: C1( -90), C2( 180))",
                "N8( 0.500,  1.866: C1(  60), C3( 120))",
                "N9( 0.500,  0.134: C1( -60), C3(-120))",
                "N10( 1.866,  0.500: C2(  30), C3( -30))",
                "N11( 0.134,  0.500: C2( 150), C3(-150))",
            ]]
        );
    }

    #[test]
    fn test_4_circles_diamond() {
        let sq3 = 3_f64.sqrt();
        let shapes = vec![
            xyrr(0. ,  0.      , 1., 1.),
            xyrr(1. ,  0.      , 1., 1.),
            xyrr(0.5,  sq3 / 2., 1., 1.),
            xyrr(0.5, -sq3 / 2., 1., 1.),
        ];
        let scene = Scene::new(shapes).unwrap();
        assert_node_strs(
            &scene,
            vec![vec![
                "N0( 0.500,  0.866: C0(  60), C1( 120))",
                "N1( 0.500, -0.866: C0( -60), C1(-120))",
                "N2( 1.000,  0.000: C0(   0), C2( -60), C3(  60))",
                "N3(-0.500,  0.866: C0( 120), C2( 180))",
                "N4(-0.500, -0.866: C0(-120), C3( 180))",
                "N5( 1.500,  0.866: C1(  60), C2(   0))",
                "N6(-0.000,  0.000: C1(-180), C2(-120), C3( 120))",
                "N7( 1.500, -0.866: C1( -60), C3(   0))",
            ]]
        );
        assert_eq!(scene.components.len(), 1);
        let component = scene.components[0].clone();
        assert_eq!(component.regions.len(), 11);
    }

    fn assert_node_strs<D: DisplayNum>(scene: &Scene<D>, expected: Vec<Vec<&str>>) {
        let gen_vals = env::var("GEN_VALS").map(|s| s.parse::<usize>().unwrap()).ok();
        match gen_vals {
            Some(_) => {
                println!("Nodes: vec![");
                for component in &scene.components {
                    println!("  vec![");
                    for node in &component.nodes {
                        println!("    {:?},", format!("{}", node.borrow()));
                    }
                    println!("  ],");
                }
                println!("]");
            },
            None => {
                let actual: Vec<Vec<String>> = scene.components.iter().map(|c| c.nodes.iter().map(|n| format!("{}", n.borrow())).collect::<Vec<String>>()).collect();
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn tweaked_2_ellipses_f64() {
        // find_roots_quartic bug?
        // a_4: 0.000000030743755847066437
        // a_3: 0.000000003666731306801131
        // a_2: 1.0001928389119579
        // a_1: 0.000011499702220469921
        // a_0: -0.6976068572771268
        // Roots: Two([-5.106021787341158, 5.106010194296296])
        //   x: -5.106021787341158, f(x): 25.37884091853862 ❌ should be 0!
        //   x: 5.106010194296296, f(x): 25.37884091853862  ❌ should be 0!
        // Actual roots ≈ ±0.835153846196954, per:
        // https://www.wolframalpha.com/input?i=0.000000030743755847066437+x%5E4++%2B0.000000003666731306801131+x%5E3+%2B+1.0001928389119579+x%5E2++%2B0.000011499702220469921+x+-+0.6976068572771268
        let shapes = vec![
            xyrr(0.3472135954999579, 1.7888543819998317, 1.0,                2.0               ),
            xyrr(1.4472087032327248, 1.7888773809286864, 0.9997362494738584, 1.9998582057729295),
        ];
        let scene = Scene::new(shapes).unwrap();
        assert_node_strs(
            &scene,
            vec![vec![
                "N0( 0.897,  3.459: C0(  57), C1( 123))",
                "N1( 0.897,  0.119: C0( -57), C1(-123))",
            ]]
        );
    }

    #[test]
    fn tweaked_3_ellipses_f64() {
        let shapes = vec![
            xyrr(0.3472135954999579, 1.7888543819998317, 1.0,                2.0               ),
            xyrr(1.4472087032327248, 1.7888773809286864, 0.9997362494738584, 1.9998582057729295),
            xyrr(1.7890795512191124, 1.4471922162722848, 1.9998252659224116, 0.9994675708661026),
        ];
        let scene = Scene::new(shapes).unwrap();
        assert_node_strs(
            &scene,
            vec![vec![
                "N0( 0.897,  3.459: C0(  57), C1( 123))",
                "N1( 0.897,  0.119: C0( -57), C1(-123))",
                "N2( 1.297,  2.416: C0(  18), C2( 104))",
                "N3( 1.115,  0.506: C0( -40), C2(-110))",
                "N4( 2.399,  2.399: C1(  18), C2(  72))",
                "N5( 2.198,  0.469: C1( -41), C2( -78))",
                "N6( 0.632,  0.632: C1(-145), C2(-125))",
                "N7( 0.469,  2.198: C1( 168), C2( 131))",
            ]]
        );
    }

    #[test]
    fn fizz_bazz_buzz_qux_error() {
        let shapes = vec![
            xyrr( -2.0547374714862916,  0.7979432881804286  , 15.303664487498873, 17.53077114567813 ),
            xyrr(-11.526407092112622 ,  3.0882189920409058  , 22.75383340199038 ,  5.964648612528639),
            xyrr( 10.550418544451459 ,  0.029458342547552023,  6.102407875525676, 11.431493472697646),
            xyrr(  4.271631577807546 , -5.4473446956862155  ,  2.652054463066812, 10.753963707585315),
        ];
        let scene = Scene::new(shapes).unwrap();
        assert_eq!(scene.components.len(), 1);
        let component = scene.components[0].clone();
        assert_node_strs(
            &scene,
            vec![vec![
                "N0(-15.600,  8.956: C0( 152), C1( 100))",
                "N1(-17.051, -2.698: C0(-168), C1(-104))",
                "N2( 10.112,  11.431: C0(  37), C2(  94))",
                "N3( 9.177, -11.109: C0( -43), C2(-103))",
                "N4( 5.795, -14.251: C0( -59), C3( -55))",
                "N5( 3.388, -15.587: C0( -69), C3(-109))",
                "N6( 5.707,  6.983: C1(  41), C2( 143))",
                "N7( 4.481, -1.151: C1( -45), C2(-174))",
                "N8( 6.627, -0.508: C1( -37), C3(  27))",
                "N9( 1.781, -1.750: C1( -54), C3( 160))",
                "N10( 5.022,  4.868: C2( 155), C3(  74))",
                "N11( 6.778, -8.957: C2(-128), C3( -19))",
            ]]
        );
        assert_eq!(component.regions.len(), 13);
    }

    #[test]
    fn disjoint() {
        let shapes = vec![
            xyrr(0., 0., 1., 1.,),
            xyrr(3., 0., 1., 1.,),
            xyrr(0., 3., 1., 1.,),
            xyrr(3., 3., 1., 1.,),
        ];
        let scene = Scene::new(shapes).unwrap();
        assert_eq!(scene.components.len(), 4);
    }

    fn verify_singleton_component(component: &Component<f64>, num_child_components: usize, area: f64) {
        assert_eq!(component.regions.len(), 1);
        let region = component.regions[0].clone();
        assert_eq!(region.child_components.len(), num_child_components);
        assert_eq!(component.hull.area(), area);
    }

    #[test]
    fn containment_2() {
        let shapes = vec![
            xyrr(0., 0., 1., 1.,),
            xyrr(0., 0., 2., 2.,),
        ];
        fn check(shapes: Vec<Shape<f64>>) {
            let scene = Scene::new(shapes).unwrap();
            assert_eq!(scene.components.len(), 2);
            for component in &scene.components {
                assert_eq!(component.nodes.len(), 1);
                assert_eq!(component.edges.len(), 1);
                assert_eq!(component.regions.len(), 1);
            }
            verify_singleton_component(&scene.components[0], 1, 4. * PI);
            verify_singleton_component(&scene.components[1], 0, 1. * PI);
        }
        shapes.into_iter().permutations(2).for_each(|shapes| check(shapes.clone()));
    }

    #[test]
    fn containment_3() {
        let shapes = vec![
            xyrr(0., 0., 1., 1.,),
            xyrr(0., 0., 2., 2.,),
            xyrr(0., 0., 3., 3.,),
        ];
        fn check(shapes: Vec<Shape<f64>>) {
            let scene = Scene::new(shapes).unwrap();
            assert_eq!(scene.components.len(), 3);
            for component in &scene.components {
                assert_eq!(component.nodes.len(), 1);
                assert_eq!(component.edges.len(), 1);
                assert_eq!(component.regions.len(), 1);
            }
            verify_singleton_component(&scene.components[0], 1, 9. * PI);
            verify_singleton_component(&scene.components[1], 1, 4. * PI);
            verify_singleton_component(&scene.components[2], 0, 1. * PI);
        }
        shapes.into_iter().permutations(3).for_each(|shapes| check(shapes.clone()));
    }

    #[test]
    fn containment_4() {
        let shapes = vec![
            xyrr(0., 0., 1., 1.,),
            xyrr(0., 0., 2., 2.,),
            xyrr(0., 0., 3., 3.,),
            xyrr(0., 0., 4., 4.,),
        ];
        fn check(shapes: Vec<Shape<f64>>) {
            let scene = Scene::new(shapes).unwrap();
            assert_eq!(scene.components.len(), 4);
            for component in &scene.components {
                assert_eq!(component.nodes.len(), 1);
                assert_eq!(component.edges.len(), 1);
                assert_eq!(component.regions.len(), 1);
            }
            verify_singleton_component(&scene.components[0], 1, 16. * PI);
            verify_singleton_component(&scene.components[1], 1,  9. * PI);
            verify_singleton_component(&scene.components[2], 1,  4. * PI);
            verify_singleton_component(&scene.components[3], 0,  1. * PI);
        }
        shapes.into_iter().permutations(4).for_each(|shapes| check(shapes.clone()));
    }

    #[test]
    fn containment_2_1() {
        let shapes = vec![
            xyrr(-1., 0., 2., 2.,),
            xyrr( 1., 0., 2., 2.,),
            xyrr( 0., 0., 0.5, 0.5,),
        ];
        let scene = Scene::new(shapes).unwrap();
        assert_eq!(scene.components.len(), 2);
        let c0 = &scene.components[0];
        c0.regions.iter().for_each(|r| {
            if r.key == "01-" {
                assert_eq!(r.child_components.len(), 1);
                assert_eq!(r.child_components[0].borrow().key.0, "2");
            } else {
                assert_eq!(r.child_components.len(), 0);
            }
        });
    }

    #[test]
    fn webapp_bug2() {
        let shapes = vec![
            xyrrt(-0.5732201984921728   , 0.017849273613185838, 1.0718496789749037, 1.1239177026091387, -2.2393818587498726e-17 ),
            xyrrt(-6.497191209429278e-17, 0.8303292848048783  , 0.9719623398875452, 0.9719624265723679, -5.780703446323498e-17  ),
            xyrrt( 0.5732201984921729   , 0.01784927361318584 , 0.8829677826453782, 0.9350358062796132,  0.0                    ),
        ];
        let scene = Scene::new(shapes).unwrap();
        assert_eq!(scene.components.len(), 1);
        let component = &scene.components[0];
        assert_eq!(component.nodes.len(), 6);
        assert_eq!(component.edges.len(), 12);
        assert_eq!(component.regions.len(), 7);
    }
}
