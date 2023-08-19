
pub fn round(f: &f64) -> i64 {
    if f >= &0. {
        (f + 0.5) as i64
    } else {
        (f - 0.5) as i64
    }
}

