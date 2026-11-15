use super::*;

const SOURCE: &str = r#"
use std::collections::BTreeSet;
pub struct Dog { pub weight: u32 }
pub struct Cat { pub weight: u32 }
impl Dog { pub fn heavy(&self) -> bool { self.weight > 30 } }
impl Cat { pub fn plump(&self) -> bool { self.weight > 6 } }
pub fn heavy_dogs(dogs: &[Dog], limit: usize) -> usize {
    dogs.iter().filter(|dog| dog.heavy()).take(limit).count()
}
pub fn plump_cats(cats: &[Cat], limit: usize) -> usize {
    cats.iter().filter(|cat| cat.plump()).take(limit).count()
}
pub fn light_dogs(dogs: &[Dog], limit: usize) -> usize {
    dogs.iter().filter(|dog| dog.weight + 3 < 30 && dog.weight % 2 == 0).take(limit).count()
}
pub struct Edge { from: usize, to: usize, kind: u8, position: usize }
pub fn borrowed(values: &[String]) -> Vec<&String> {
    let mut seen = BTreeSet::new();
    values.iter().filter(|v| seen.insert(*v)).collect()
}
pub fn owned(values: Vec<Edge>) -> Vec<Edge> {
    let mut seen = BTreeSet::new();
    values.into_iter().filter(|e| seen.insert((e.from,e.to,e.kind,e.position))).collect()
}
"#;

/// A closure's body is part of its function's graph. What two closures call is a hole of their
/// functions' candidate, and a function whose closure does something else isn't in it, where
/// all three read alike while a closure was one node. Two functions that only share the calls
/// around unlike closures are no candidate at all.
#[test]
fn closure_bodies_are_compared_in_compiler_proposals_and_saved_replay() {
    let (directory, _output, original) = capture(SOURCE, &["--compiler-patterns"]);
    let evaluation = &proposals(&original)["evaluation"];
    let shared = candidate_at(evaluation, "heavy_dogs").expect("a candidate for the two filters");
    assert_eq!(shared["kind"], "trait_method");
    assert_eq!(site_names(shared, "sites"), ["heavy_dogs", "plump_cats"]);
    let observation = shared["observation"].as_str().unwrap();
    assert!(
        observation.contains("M0 = Dog::heavy | Cat::plump"),
        "{observation}"
    );
    for alone in ["light_dogs", "borrowed", "owned"] {
        assert!(candidate_at(evaluation, alone).is_none(), "{alone}");
    }
    assert_eq!(evaluation, &patterns_json(directory.path(), &[]));
    for path in ["src", "Cargo.toml", "Cargo.lock"] {
        let path = directory.path().join(path);
        if path.is_dir() {
            std::fs::remove_dir_all(path).unwrap();
        } else {
            std::fs::remove_file(path).unwrap();
        }
    }
    let (_output, saved) = replay(directory.path(), &original, &[]);
    assert_eq!(original["compiler_advisory"], saved["compiler_advisory"]);
}
