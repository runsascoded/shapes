use core::panic;
use std::{collections::{HashMap, HashSet}, ops::{Sub, Add}, fmt::Display};

use num_traits::pow;

use crate::zero::Zero;

pub struct Areas<D> {
    pub map: HashMap<String, D>,
    pub n: usize,
}

type Neighbor = (char, String);
type Neighbors = Vec<(Neighbor, Neighbor)>;

impl<D: Clone + Zero<D> + Display + Add<Output = D> + Sub<Output = D>> Areas<D>
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
    pub fn expand(map: &mut HashMap<String, D>) {
        let initial_size = map.len();
        let n = map.keys().next().unwrap().len();
        let empty_key = String::from_utf8(vec![b'-'; n]).unwrap();
        if !map.contains_key(&empty_key) {
            let first = map.values().next().unwrap().clone();
            map.insert(empty_key, D::zero(&first));
        }
        let mut queue: HashSet<String> = map.keys().cloned().collect();
        let max = pow(3, n);
        let mut remaining = queue.len();
        while remaining > 0 && map.len() < max {
            let k0 = queue.clone().into_iter().next().unwrap();
            queue.remove(&k0);
            remaining -= 1;
            // println!("popped: {}, {} remaining, {} overall", k0, remaining, map.len());
            let neighbors = Areas::<D>::neighbors(&k0);
            // println!("neighbors: {:?}", neighbors);
            for (_, (((ch1, k1), (ch2, k2)), ch0)) in neighbors.iter().zip(k0.chars()).enumerate() {
                let k0 = k0.clone();
                let v0 = map.get( &k0);
                let v1 = map.get(  k1);
                let v2 = map.get(  k2);
                let keys = HashMap::from([
                    ( ch0, (k0.clone(), v0)),
                    (*ch1, (k1.clone(), v1)),
                    (*ch2, (k2.clone(), v2)),
                ]);
                // println!("keys: {} {} {}", ch0, ch1, ch2);
                let mut somes: Vec<(char, (String, &D))> = Vec::new();
                let mut nones: Vec<(char, String)> = Vec::new();
                for (ch, (k, v)) in keys.iter() {
                    match v {
                        None => nones.push((*ch, k.clone())),
                        Some(o) => somes.push((*ch, (k.clone(), o))),
                    }
                }
                let num_somes = somes.len();
                if num_somes == 2 {
                    let (some0, some1) = (somes[0].clone(), somes[1].clone());
                    let (none_ch, none_key) = nones.iter().next().unwrap();
                    let v =
                        if *none_ch == '*' {
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
                    map.insert(none_key.clone(), v.clone());
                    queue.insert(none_key.to_string());
                    remaining += 1;
                    // println!("inserted {} = {}, remaining {}", none_key, v, remaining);
                } else if num_somes == 3 {
                    // TODO: fsck
                }
            }
        }
        let m = map.len();
        if m < max {
            panic!("Only expanded from {} to {} keys, expected 3^{} = {}", initial_size, m, n, max);
        }
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
    use std::collections::HashMap;

    fn test(inputs: Vec<(&str, i64)>, expected: Vec<(&str, i64)>) {
        let inputs: Vec<(String, i64)> = inputs.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        let mut map: HashMap<String, i64> = inputs.into_iter().collect();
        super::Areas::<i64>::expand(&mut map);
        let mut items: Vec<(String, i64)> = map.into_iter().collect();
        items.sort_by_key(|(k, _)| k.clone());
        items.iter().zip(expected.iter()).enumerate().for_each(|(idx, ((ak, av), (ek, ev)))| {
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