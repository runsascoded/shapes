
#[cfg(test)]
mod tests {
    use roots::find_roots_quartic;

    fn compute(a4: f64, a3: f64, a2: f64, a1: f64, a0: f64) {
        println!("{}x⁴ + {}x³ + {}x² + {}x + {}", a4, a3, a2, a1, a0);
        let roots = find_roots_quartic(a4, a3, a2, a1, a0);
        println!("{:?}", roots);
        println!();
    }

    #[test]
    fn print_roots() {
        compute(1., 4., 6., 4., 1.);
        compute(1., 0., -6., 0., 9.);
        compute(1., 0., -1., 0., 0.);
    }
}