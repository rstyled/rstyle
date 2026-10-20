//! Independently executable semantics and reviewed pinned implementation sources.

#[path = "../../../eval/coverage-development/byte-window-oracle.rs"]
mod runtime;

#[test]
fn byte_call_authority_binds_reviewed_core_sources() {
    let output = std::process::Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let sysroot = String::from_utf8(output.stdout).unwrap();
    let root = std::path::Path::new(sysroot.trim()).join("lib/rustlib/src/rust/library/core/src");
    let digests: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!("../contracts/byte-window-core-source-v1.json")).unwrap();
    assert_eq!(digests.len(), 5);
    for (path, expected) in digests {
        let source = std::fs::read(root.join(path)).unwrap();
        assert_eq!(rstyle_pdg::concrete_types::digest(&source), expected);
    }
}
