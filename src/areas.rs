use core::panic;
use std::{collections::{HashMap, HashSet}, iter::Sum, ops::Sub, rc::Rc, cell::RefCell};

use num_traits::pow;

use crate::zero::Zero;

pub struct Areas<D> {
    pub map: HashMap<String, D>,
    pub n: usize,
}

type Neighbor = (char, String);
type Neighbors = Vec<(Neighbor, Neighbor)>;

impl<D: Clone + Zero<D> + Sum + Sub<Output = D>> Areas<D>
// where &'a D: Sub<&D>
{
    pub fn neighbor(prefix: &String, ch: char, suffix: &String) -> Neighbor {
        (ch, format!("{}{}{}", prefix, ch, suffix))
    }
    pub fn neighbors(key: &str) -> Neighbors {
        key.chars().enumerate().map(|(idx, ch)| {
            // let (prefix, suffix): (String, String) = {
            let prefix: String = key.chars().take(idx).collect();
            let suffix: String = key.chars().skip(idx + 1).collect();
                // (prefix, suffix)
            // };
            let i = Self::idx(idx);
            let chars = match ch {
                '-' => [ '*', i, ],  //(format!("{}*{}", prefix, suffix), format!("{}{}{}", prefix, i, suffix)),
                '*' => [ '-', i, ],  //(format!("{}-{}", prefix, suffix), format!("{}{}{}", prefix, i, suffix)),
                 _  => [ '-', i, ],  //(format!("{}-{}", prefix, suffix), format!("{}*{}", prefix, suffix)),
            };
            (
                Self::neighbor(&prefix, chars[0], &suffix),
                Self::neighbor(&prefix, chars[1], &suffix),
            )
        }).collect()
    }
    pub fn expand(map: &mut HashMap<String, D>) {
        let map = Rc::new(RefCell::new(map.clone()));
        let initial_size = map.borrow().len();
        let n = map.borrow().keys().next().unwrap().len();
        let empty_key = String::from_utf8(vec![b'-'; n]).unwrap();
        //let mut map = map.borrow();
        if !map.contains_key(&empty_key) {
            let first = map.values().next().unwrap().clone();
            map.insert(empty_key, D::zero(first));
        }
        let mut queue: HashSet<String> = map.keys().cloned().collect();
        let max = pow(3, n);
        let mut remaining = queue.len();
        while remaining > 0 && map.len() < max {
            let k0 = queue.clone().into_iter().next().unwrap();
            queue.remove(&k0);
            remaining -= 1;
            let neighbors = Areas::<D>::neighbors(&k0);
            for (_, (((ch1, k1), (ch2, k2)), ch0)) in neighbors.iter().zip(k0.chars()).enumerate() {
                let keys = HashMap::from([
                    ( ch0, (k0.clone(), map.get(&k0))),
                    (*ch1, (k1.clone(), map.get( k1))),
                    (*ch2, (k2.clone(), map.get( k2))),
                ]);
                let (somes, nones): (Vec<_>, Vec<_>) = keys.into_iter().partition(|(_, (_, v))| v.is_some());
                let num_somes = somes.len();
                if num_somes == 2 {
                    let (some0, some1) = (somes[0].clone(), somes[1].clone());
                    let (none_ch, (none_key, _)) = nones.iter().next().unwrap();
                    let v =
                        if none_ch == &'*' {
                            somes.iter().map(|(_, (_, v))| v.unwrap().clone()).sum::<D>()
                        } else {
                            let ((_, (_, all_val)), (_, (_, other_val))) =
                                if somes[0].0 == '*' {
                                    (some0, some1)
                                } else {
                                    (some1, some0)
                                };
                            all_val.unwrap().clone() - other_val.unwrap().clone()
                        };
                    map.insert(none_key.clone(), v);
                    queue.insert(none_key.to_string());
                    remaining += 1;
                }
            }
        }
        let m = map.len();
        if m < max {
            panic!("Only expanded from {} to {} keys, expected 3^{} = {}", initial_size, m, n, max);
        }
    }

    pub fn idx(idx: usize) -> char {
        assert!(idx < 10);
        format!("{}", idx).chars().next().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    #[test]
    fn expand() {
        let mut map: HashMap<String, i64> = HashMap::from([
            ("0*".to_string(), 9),
            ("*1".to_string(), 3),
            ("01".to_string(), 1),
        ]);
        super::Areas::<i64>::expand(&mut map);
        assert_eq!(map.len(), 9);
    }
}