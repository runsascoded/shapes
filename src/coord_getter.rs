use std::ops::Deref;

pub struct CoordGetter<In> {
    pub name: String,
    pub get: Box<dyn Fn(In) -> f64>,
}

impl<'a, In: 'a> Deref for CoordGetter<In> {
    type Target = Box<dyn Fn(In) -> f64>;
    fn deref(&self) -> &Self::Target {
        &self.get
    }
}

impl<In> CoordGetter<In> {
    pub fn new(name: &str, get: Box<dyn Fn(In) -> f64>) -> Self {
        Self { name: name.to_string(), get }
    }
}

impl<'a: 'static, In: 'a> CoordGetter<In> {
    pub fn map<New: 'a>(&'a self, f: Box<dyn Fn(New) -> In>) -> CoordGetter<New> {
        CoordGetter {
            name: self.name.clone(),
            get: Box::new(move |new: New| (self.get)(f(new))),
        }
    }
}

pub fn coord_getter<In, T: Fn(In) -> f64 + 'static>(name: &str, get: T) -> CoordGetter<In> {
    CoordGetter::new(name, Box::new(get))
}
