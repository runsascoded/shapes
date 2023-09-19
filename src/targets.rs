use core::panic;
use std::{collections::{BTreeMap, BTreeSet}, ops::{Sub, Add, Deref}, fmt::Display};

use num_traits::pow;
use serde::{Serialize, Deserialize};
use tsify::{declare, Tsify};

use crate::zero::Zero;

#[declare]
pub type TargetsMap<D> = BTreeMap<String, D>;

#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
pub struct Targets<D> {
    pub all: TargetsMap<D>,
    pub given: BTreeSet<String>,
    pub n: usize,
    pub total_area: D,
}

type Neighbor = (char, String);
type Neighbors = Vec<(Neighbor, Neighbor)>;

impl<D> Deref for Targets<D> {
    type Target = TargetsMap<D>;
    fn deref(&self) -> &Self::Target {
        &self.all
    }
}

pub trait Arg
: Copy
+ Zero
+ Display
+ Add<Output = Self>
+ Sub<Output = Self>
{}
impl Arg for f64 {}
impl Arg for i64 {}

impl<D: Arg> From<TargetsMap<D>> for Targets<D> {
    fn from(given: TargetsMap<D>) -> Self {
        Self::new(given)
    }
}

impl<D: Arg> Targets<D>
{
    pub fn neighbor(prefix: &String, ch: char, suffix: &String) -> Neighbor {
        (ch, format!("{}{}{}", prefix, ch, suffix))
    }
    pub fn neighbors(key: &str) -> Neighbors {
        key.chars().enumerate().map(|(idx, ch)| {
            let prefix: String = key.chars().take(idx).collect();
            let suffix: String = key.chars().skip(idx + 1).collect();
            let i = Self::idx(idx);
            let chars = match ch {
                '-' => [ '*',  i , ],
                '*' => [ '-',  i , ],
                 _  => [ '-', '*', ],
            };
            (
                Self::neighbor(&prefix, chars[0], &suffix),
                Self::neighbor(&prefix, chars[1], &suffix),
            )
        }).collect()
    }
    pub fn new(given: TargetsMap<D>) -> Targets<D> {
        let mut all = given.clone();
        let initial_size = all.len();
        let n = all.keys().next().unwrap().len();
        let empty_key = String::from_utf8(vec![b'-'; n]).unwrap();
        if !all.contains_key(&empty_key) {
            let first = all.values().next().unwrap();
            all.insert(empty_key, D::zero(&first));
        }
        let mut queue: BTreeSet<String> = all.keys().cloned().collect();
        let max = pow(3, n);
        let mut remaining = queue.len();
        while remaining > 0 && all.len() < max {
            // let k0 = queue.iter().next().unwrap();
            // queue.remove(k0);
            let k0 = queue.pop_first().unwrap();
            remaining -= 1;
            // println!("popped: {}, {} remaining, {} overall", k0, remaining, map.len());
            let neighbors = Targets::<D>::neighbors(&k0);
            // println!("neighbors: {:?}", neighbors);
            for (_, (((ch1, k1), (ch2, k2)), ch0)) in neighbors.into_iter().zip((&k0).chars()).enumerate() {
                let (somes, nones) = {
                    let keys = {
                        let v0 = all.get(&k0);
                        let v1 = all.get(&k1);
                        let v2 = all.get(&k2);
                        let keys = BTreeMap::from([
                            (ch0, (k0.clone(), v0)),
                            (ch1, (k1.clone(), v1)),
                            (ch2, (k2.clone(), v2)),
                        ]);
                        keys
                    };
                    // println!("keys: {} {} {}", ch0, ch1, ch2);
                    let mut somes: Vec<(char, (String, &D))> = Vec::new();
                    let mut nones: Vec<(char, String)> = Vec::new();
                    for (ch, (k, v)) in keys.into_iter() {
                        match v {
                            Some(o) => somes.push((ch, (k, o))),
                            None => nones.push((ch, k)),
                        }
                    }
                    (somes, nones)
                };
                let num_somes = somes.len();
                if num_somes == 2 {
                    let (some0, some1) = (somes[0].clone(), somes[1].clone());
                    let (none_ch, none_key) = nones.into_iter().next().unwrap();
                    let v =
                        if none_ch == '*' {
                            let ((_, (_, some0v)), (_, (_, some1v))) = (some0, some1);
                            some0v.clone() + some1v.clone()
                        } else {
                            let ((_, (_, all_val)), (_, (_, other_val))) =
                                if somes[0].0 == '*' {
                                    (some0, some1)
                                } else {
                                    (some1, some0)
                                };
                            all_val.clone() - other_val.clone()
                        };
                    all.insert(none_key.clone(), v);
                    queue.insert(none_key);
                    remaining += 1;
                    // println!("inserted {} = {}, remaining {}", none_key, v, remaining);
                } else if num_somes == 3 {
                    // TODO: fsck
                }
            }
        }
        let m = all.len();
        if m < max {
            panic!("Only expanded from {} to {} keys, expected 3^{} = {}", initial_size, m, n, max);
        }

        let all_key = String::from_utf8(vec![b'*'; n]).unwrap();
        let total_area =
            all
            .get(&all_key)
            .expect(&format!("{} not found among {} keys", all_key, all.len()))
            .clone();

        Targets {
            all,
            given: given.keys().cloned().collect(),
            n,
            total_area,
        }
    }
    pub fn disjoints(&self) -> TargetsMap<D> {
        let mut map: TargetsMap<D> = BTreeMap::new();
        self.disjoints_rec(String::new(), &mut map);
        let none_key = self.none_key();
        map.into_iter().filter(|(k, _)| k != &none_key).collect()
    }
    pub fn disjoints_rec(&self, prefix: String, map: &mut TargetsMap<D>) {
        let idx = prefix.len();
        if idx == self.n {
            let value = self.all.get(&prefix).unwrap();
            map.insert(prefix, value.clone());
        } else {
            self.disjoints_rec(format!("{}{}", prefix, '-'), map);
            self.disjoints_rec(format!("{}{}", prefix, Targets::<D>::idx(idx)), map);
        }
    }
}
impl<D> Targets<D> {
    pub fn none_key(&self) -> String {
        String::from_utf8(vec![b'-'; self.n]).unwrap()
    }
    pub fn idx(idx: usize) -> char {
        if idx < 10 {
            char::from_digit(idx as u32, 10).unwrap()
        } else if idx < 36 {
            char::from_u32('a' as u32 + (idx - 11) as u32).unwrap()
        } else {
            panic!("idx {} out of range, maximum 36 sets supported", idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test(inputs: Vec<(&str, i64)>, expected: Vec<(&str, i64)>) {
        let inputs: Vec<(String, i64)> = inputs.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        let map: TargetsMap<i64> = inputs.into_iter().collect();
        let targets = Targets::<i64>::new(map);
        let items: Vec<(String, i64)> = targets.all.into_iter().collect();
        // items.sort_by_key(|(k, _)| k.clone());
        items.iter().zip(expected.iter()).for_each(|((ak, av), (ek, ev))| {
            assert_eq!(ak, ek);
            if av != ev {
                println!("  {}: {} != {}", ak, av, ev);
            }
        });
        assert_eq!(items.len(), expected.len());
        assert_eq!(items, expected.into_iter().map(|(k, v)| (k.to_string(), v)).collect::<Vec<_>>());
    }

    #[test]
    fn expand2() {
        test(vec![
            ("0*", 9),
            ("*1", 3),
            ("01", 1),
        ], vec![
            ("**", 11),
            ("*-",  8),
            ("*1",  3),
            ("-*",  2),
            ("--",  0),
            ("-1",  2),
            ("0*",  9),
            ("0-",  8),
            ("01",  1),
        ]);
    }

    #[test]
    fn expand3() {
        test(vec![
            ("0**",  9),
            ("*1*",  9),
            ("**2",  9),
            ("01*",  3),
            ("0*2",  3),
            ("*12",  3),
            ("012",  1),
        ], vec![
            ("***", 19),
            ("**-", 10),
            ("**2",  9),
            ("*-*", 10),
            ("*--",  4),
            ("*-2",  6),
            ("*1*",  9),
            ("*1-",  6),
            ("*12",  3),

            ("-**", 10),
            ("-*-",  4),
            ("-*2",  6),
            ("--*",  4),
            ("---",  0),
            ("--2",  4),
            ("-1*",  6),
            ("-1-",  4),
            ("-12",  2),

            ("0**",  9),
            ("0*-",  6),
            ("0*2",  3),
            ("0-*",  6),
            ("0--",  4),
            ("0-2",  2),
            ("01*",  3),
            ("01-",  2),
            ("012",  1),
        ]);
    }
}