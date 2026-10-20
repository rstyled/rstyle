use super::*;

#[test]
fn end_of_input_source_first_baseline_preserves_controls_and_complete_inventory() {
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
            let directory = clone_workspace();
            let source = include_str!("../../../../eval/coverage-development/end-of-input.rs");
            let source = format!("{}{}", if no_std { "#![no_std]\n" } else { "" }, source);
            std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
            let (output, report) = clones(
                directory.path(),
                &["--extraction-profile", "hir-data-v3", "--target", target],
            );
            assert!(output.status.success(), "{report}");
            check_baseline(&report);
        }
    }
}

fn check_baseline(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 13, "{report}");
    for function in functions {
        let admitted = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "Unit::as_usize" | "ByteClasses::empty" | "ByteClasses::alphabet_len"
        );
        assert_eq!(
            function["outcome"],
            if admitted { "admitted" } else { "excluded" },
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
