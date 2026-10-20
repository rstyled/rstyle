//! Source-selected expectations precede loop/merge support.
use super::*;

fn natural_gcd() -> &'static str {
    let source = include_str!("../../../rstyle-ccgraph/src/frac.rs");
    let start = source.find("fn gcd(").unwrap();
    let end = source[start..].find("\nfn split_decimal(").unwrap();
    source[start..start + end].trim()
}

#[test]
fn loop_merge_target_retains_complete_baseline() {
    let directory = clone_workspace();
    let source = format!(
        "{}\n{}",
        include_str!("../../../../eval/coverage-development/loop-merges.rs"),
        natural_gcd()
    );
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 7, "{report}");
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        assert_eq!(
            function["outcome"],
            if name == "sequential" {
                "admitted"
            } else {
                "excluded"
            },
            "{function}"
        );
        if matches!(name, "gcd" | "after_loop" | "before_loop" | "cyclic_merge") {
            assert!(
                function["reasons"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|reason| { reason.as_str().unwrap().contains("cyclic policy excludes") }),
                "{function}"
            );
        }
    }
    check_aggregate_documents(&report);
}

#[test]
fn loop_merge_target_has_independent_runtime_expectations() {
    let directory = tempfile::tempdir().unwrap();
    let source = format!(
        "{}\n{}\n{}",
        include_str!("../../../../eval/coverage-development/loop-merges.rs"),
        natural_gcd(),
        include_str!("../../../../eval/coverage-development/loop-merge-oracle.rs")
            .lines()
            .filter(|line| !line.starts_with("//!"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let path = directory.path().join("oracle.rs");
    let binary = directory.path().join("oracle");
    std::fs::write(&path, source).unwrap();
    let output = Command::new("rustc")
        .args(["--edition=2021", "--test", "-C", "overflow-checks=on"])
        .arg(&path)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}
