mod corruption;
use super::*;

#[test]
fn byte_comparison_family_composes_through_strict_matching() {
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
            eprintln!("target std not installed: {target}");
            continue;
        }
        let (_directory, report) = captured_family(target);
        check_family(&report);
    }
}

fn captured_family(target: &str) -> (tempfile::TempDir, Value) {
    let directory = clone_workspace();
    let source = format!(
        "{}\n{}",
        include_str!("../../../../tests/fixtures/function-families/tagged-units.rs"),
        include_str!("../../../../tests/fixtures/function-families/tagged-byte-comparisons.rs")
    );
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--target", target],
    );
    assert!(output.status.success(), "{report}");
    (directory, report)
}

#[test]
fn callback_runtime_oracle_keeps_independent_eager_and_sentinel_expectations() {
    let directory = clone_workspace();
    let source = format!(
        "{}\n{}\n{}",
        include_str!("../../../../tests/fixtures/function-families/tagged-units.rs"),
        include_str!("../../../../tests/fixtures/function-families/tagged-byte-comparisons.rs"),
        include_str!(
            "../../../../tests/fixtures/function-families/tagged-byte-comparisons-oracle.rs"
        )
    );
    let path = directory.path().join("src/lib.rs");
    std::fs::write(&path, source).unwrap();
    let executable = directory.path().join("oracle");
    let compiled = Command::new("rustc")
        .args(["--edition=2021", "--test"])
        .arg(&path)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(compiled.status.success(), "{compiled:?}");

    let runtime = Command::new(executable).output().unwrap();
    assert!(runtime.status.success(), "{runtime:?}");
}

fn check_family(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 26);
    for function in functions {
        let expected_positive = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "Unit::new_byte"
                | "Unit::new_end"
                | "Unit::byte"
                | "Unit::end"
                | "roundtrip"
                | "saved"
                | "Unit::is_byte"
                | "byte_matches"
                | "end_never_matches"
                | "saved_comparison"
                | "local_option"
                | "default_before_capture"
                | "none_uses_default"
        );
        assert_eq!(
            function["outcome"],
            if expected_positive {
                "admitted"
            } else {
                "excluded"
            },
            "{function}"
        );
    }
    check_aggregate_documents(report);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
}
