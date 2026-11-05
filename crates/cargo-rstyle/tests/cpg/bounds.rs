//! Runtime-bound programs lowered from `compat.cpg` documents, over the advisory
//! fixture written to exercise each branch of the lowering.

use super::*;

/// The fixture's emitted documents.
fn emitted() -> (TempDir, PathBuf) {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        crate::compiler_advisory::runtime_bounds::SOURCE,
    )
    .unwrap();
    let out = directory.path().join("cpg");
    let output = rstyle(
        directory.path(),
        &["rstyle", "cpg", "emit", "--out", out.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    (directory, out)
}

/// The programs the lowering derives, frozen from the driver's own HIR lowering when the
/// two were proven equal: one canonical program per function.
#[test]
fn the_cpg_lowering_keeps_the_frozen_runtime_bound_programs() {
    let (_directory, out) = emitted();
    let document = std::fs::read_dir(&out)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.to_string_lossy().ends_with(".cpg.json"))
        .unwrap();
    let document: Value = serde_json::from_slice(&std::fs::read(document).unwrap()).unwrap();
    // The crate's stable id names its definitions and depends on where it was built.
    let compilation = document["unit"]["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|definition| definition["scope"] == "unit")
        .find_map(|definition| definition["compilation"].as_str())
        .unwrap()
        .to_string();
    let programs = rstyle_core::cpg::run_bounds(vec![document]).unwrap();
    let lines: Vec<String> = programs
        .iter()
        .map(|(function, input)| {
            let input = serde_json::to_value(input).unwrap();
            let canonical = rstyle_core::cpg::bounds::canonical_bounds(&input);
            format!("{function} {}", canonical.replace(&compilation, "<crate>"))
        })
        .collect();
    insta::assert_snapshot!(lines.join("\n"));
}
