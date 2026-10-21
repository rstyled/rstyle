use super::*;

const TABLE: &str = include_str!("../../../../eval/coverage-development/word-byte-classifiers.rs");
const SCALAR: &str =
    include_str!("../../../../eval/coverage-development/byte-scalar-classifiers.rs");

#[test]
fn word_classifier_source_first_selections_keep_complete_blocker_chains() {
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
            check_context(target, no_std, TABLE);
            check_context(target, no_std, SCALAR);
        }
    }
}

fn check_context(target: &str, no_std: bool, source: &str) {
    let directory = clone_workspace();
    let bytes = format!("{}{}", if no_std { "#![no_std]\n" } else { "" }, source);
    std::fs::write(directory.path().join("src/lib.rs"), bytes).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--target", target],
    );
    assert!(output.status.success(), "{}", report["capture_failures"]);
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), if source == TABLE { 20 } else { 11 });
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        let admitted =
            source == TABLE && matches!(name, "Unit::new_byte" | "Unit::new_end" | "Unit::byte");
        assert_eq!(
            function["outcome"],
            if admitted { "admitted" } else { "excluded" },
            "{target}, no_std={no_std}, {name}: {}",
            function["reasons"]
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
    check_aggregate_documents(&report);
}
