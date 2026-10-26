//! Repeated observations reuse work without granting authority or hiding limits.
use super::*;
use rstyle_pdg::concrete_types::{InterfaceObservations, Observation};

fn wide_source() -> String {
    let fields = (0..16)
        .map(|index| format!("f{index}: u8"))
        .collect::<Vec<_>>()
        .join(",");
    format!("pub struct Leaf {{ {fields} }}\npub struct Pair {{ left: Leaf, right: Leaf }}\n")
}

#[test]
fn repeated_concrete_observations_preserve_every_source_operation() {
    let directory = clone_workspace();
    let mut source = wide_source();
    source.push_str("pub fn repeated(a: &Pair) -> usize {\n");
    for _ in 0..160 {
        source.push_str("let _ = a;\n");
    }
    source.push_str("core::hint::black_box(a); 0 }\n");
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    let entry = float_kernels::function(&report, "repeated");
    // Reusing a type observation does not certify the unregistered call.
    assert_eq!(entry["outcome"], "excluded");
    let (store, index) = shared_evidence::load(entry);
    let facts = &store.functions()[index];
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()
        .unwrap();
    assert!(matches!(
        interface.parameters[0],
        Observation::Captured { .. }
    ));
    interface.validate().unwrap();

    let mut nodes = Vec::new();
    descendants(&facts.body, &mut nodes);
    let reads: Vec<_> = nodes
        .into_iter()
        .filter(|node| node.kind == "expr_path" && node.snippet == "a")
        .collect();
    assert_eq!(reads.len(), 161);
    assert!(
        reads
            .iter()
            .all(|node| node.ty.as_ref().unwrap().normalized.is_some())
    );
}

#[test]
fn root_observation_reuse_does_not_authorize_nested_root_only_types() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn root_then_nested(c: char, nested: (char,)) -> char { let _ = nested; c }",
    )
    .unwrap();

    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    let entry = float_kernels::function(&report, "root_then_nested");
    assert_eq!(entry["outcome"], "excluded");
    let (store, index) = shared_evidence::load(entry);
    let interface = store.functions()[index]
        .build
        .frontend_options
        .data::<InterfaceObservations>()
        .unwrap();
    assert!(matches!(
        interface.parameters[0],
        Observation::Captured { .. }
    ));
    assert!(matches!(
        interface.parameters[1],
        Observation::Unsupported { .. }
    ));
}

#[test]
fn genuinely_large_concrete_observation_remains_incomplete() {
    let mut source = String::from("type T0 = u8;\n");
    for index in 1..=12 {
        source.push_str(&format!(
            "type T{index} = (T{}, T{});\n",
            index - 1,
            index - 1
        ));
    }
    source.push_str("pub fn oversized(_: T12) {}\n");
    incomplete(&source);
}

#[test]
fn reused_interface_occurrences_still_count_toward_inventory_budget() {
    let mut source = wide_source();
    let parameters = (0..120)
        .map(|index| format!("p{index}: &Pair"))
        .collect::<Vec<_>>()
        .join(",");
    source.push_str(&format!("pub fn oversized({parameters}) {{}}\n"));
    incomplete(&source);
}

fn incomplete(source: &str) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();

    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        report["capture_failures"]
    );
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["assessment_complete"], false);
    assert!(
        report["capture_failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| {
                failure
                    .as_str()
                    .unwrap()
                    .contains("Concrete type observation exceeds depth/entry budget")
            })
    );
}
