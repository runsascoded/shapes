use crate::scene::Scene;
use crate::shape::Shape;
use std::collections::BTreeMap;

use super::five_blobs::five_shape_layout;

#[test]
fn check_five_blobs_fragmentation() {
    let configs: Vec<(&str, Vec<Shape<f64>>)> = vec![
        ("parametric n=12", five_shape_layout(12, 0.10, 0.70, 1.3, 0.25)),
        ("parametric n=15", five_shape_layout(15, 0.15, 1.40, 2.1, 0.15)),
    ];

    for (label, shapes) in &configs {
        let scene: Scene<f64> = Scene::new(shapes.clone()).expect("Failed to create scene");

        let mut key_counts: BTreeMap<String, Vec<f64>> = BTreeMap::new();
        let mut total_geometric = 0usize;
        for component in &scene.components {
            for region in &component.regions {
                total_geometric += 1;
                key_counts.entry(region.key.clone()).or_default().push(region.area());
            }
        }

        let n_unique_keys = key_counts.len();
        let fragmented: Vec<_> = key_counts.iter()
            .filter(|(_, areas)| areas.len() > 1)
            .map(|(key, areas)| format!("  {} -> {} fragments: {:?}", key, areas.len(), areas))
            .collect();

        eprintln!("\n=== {} ===", label);
        eprintln!("Components: {}", scene.components.len());
        eprintln!("Total geometric regions: {}", total_geometric);
        eprintln!("Unique region keys: {}", n_unique_keys);
        if fragmented.is_empty() {
            eprintln!("No fragmentation!");
        } else {
            eprintln!("FRAGMENTED keys ({}):", fragmented.len());
            for f in &fragmented {
                eprintln!("{}", f);
            }
        }
    }
}
