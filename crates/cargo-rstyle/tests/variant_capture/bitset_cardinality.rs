use super::*;

#[test]
fn bitset_cardinality_composes_through_strict_capture_and_matching() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let library = Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(library.status.success());
        if !Path::new(String::from_utf8_lossy(&library.stdout).trim()).is_dir() {
            continue;
        }
        for no_std in [false, true] {
            let (_directory, report) = captured_context(Some(target), no_std);
            check_family(&report);
        }
    }
}

fn captured(target: Option<&str>) -> (tempfile::TempDir, Value) {
    captured_context(target, false)
}

fn captured_context(target: Option<&str>, no_std: bool) -> (tempfile::TempDir, Value) {
    let directory = clone_workspace();
    let source = include_str!("../../../../eval/coverage-development/bitset-cardinality.rs");
    let source = format!("{}{}", if no_std { "#![no_std]\n" } else { "" }, source);
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let mut flags = vec!["--extraction-profile", "hir-data-v3"];
    if let Some(target) = target {
        flags.extend(["--target", target]);
    }
    let (output, report) = clones(directory.path(), &flags);
    assert!(output.status.success(), "{report}");
    (directory, report)
}

fn check_family(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 14, "{report}");
    for function in functions {
        let positive = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "LookSet::empty"
                | "LookSet::full"
                | "LookSet::len"
                | "LookSet::is_empty"
                | "count"
                | "empty"
                | "saved"
                | "selected"
        );
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{function}"
        );
    }
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    check_aggregate_documents(report);
}

mod corruption;

#[test]
fn bitset_cardinality_does_not_certify_general_results_or_receiver_adjustments() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub fn direct(x: u32) -> usize { usize::try_from(x).unwrap() }
        pub fn saved_result(x: u32) -> usize { let result = usize::try_from(x); result.unwrap() }
        pub fn other_width(x: u64) -> usize { usize::try_from(x).unwrap() }
        pub fn fallback(x: u32) -> usize { usize::try_from(x).unwrap_or(0) }
        pub fn borrowed(x: &u32) -> u32 { x.count_ones() }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["functions"].as_array().unwrap().len(), 5);
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(
            function["outcome"],
            if function["facts"]["name"] == "direct" {
                "admitted"
            } else {
                "excluded"
            },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
}
