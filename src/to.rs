pub trait To<T1> {
    fn to(self) -> T1;
}

impl<T0, T1: From<T0>> To<Vec<T1>> for Vec<T0> {
    fn to(self: Self) -> Vec<T1> {
        self.into_iter().map(|x| x.into()).collect()
    }
}

impl To<f64> for f64 {
    fn to(self: Self) -> f64 {
        self
    }
}

impl<const N: usize, D> To<Vec<D>> for [D; N]
where D: Clone,
{
    fn to(self: Self) -> Vec<D> {
        self.to_vec()
    }
}