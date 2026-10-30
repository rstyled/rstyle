use super::*;

const SOURCE: &str = r#"
use std::collections::BTreeSet;
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

#[test]
fn opaque_closure_contracts_remain_visible_in_compiler_proposals_and_saved_replay() {
    let (directory, _output, original) =
        capture(SOURCE, &["--compiler-patterns", "--compiler-advisory-only"]);
    let evaluation = &proposals(&original)["evaluation"];
    let helper = candidate_at(evaluation, "borrowed").expect("advisory helper");
    assert_eq!(helper["kind"], "parameterize");
    assert_eq!(site_names(helper, "sites"), ["borrowed", "owned"]);
    assert!(
        helper["counter_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| {
                note.as_str()
                    .unwrap()
                    .contains("Ordinary closure bodies are opaque")
            })
    );
    assert_eq!(evaluation, &patterns_json(directory.path(), &[]));
    for path in ["src", "Cargo.toml", "Cargo.lock"] {
        let path = directory.path().join(path);
        if path.is_dir() {
            std::fs::remove_dir_all(path).unwrap();
        } else {
            std::fs::remove_file(path).unwrap();
        }
    }
    let (_output, saved) = replay(directory.path(), &original, &["--compiler-advisory-only"]);
    assert_eq!(original["compiler_advisory"], saved["compiler_advisory"]);
}
