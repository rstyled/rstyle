use super::*;

#[test]
fn accelerator_packing_source_first_baseline_preserves_complete_selection() {
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
            check_context(target, no_std);
        }
    }
}

fn check_context(target: &str, no_std: bool) {
    let directory = clone_workspace();
    let source = include_str!("../../../../eval/coverage-development/accelerator-packing.rs");
    let source = format!("{}{}", if no_std { "#![no_std]\n" } else { "" }, source);
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--target", target],
    );
    assert!(
        output.status.success(),
        "{}; {}",
        String::from_utf8_lossy(&output.stderr),
        report["matching"]["diagnostics"]
    );
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 19);
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        let expected = if matches!(name, "Accel::len" | "raw") {
            "admitted"
        } else {
            "excluded"
        };
        assert_eq!(
            function["outcome"], expected,
            "{target} {name}: {}",
            function["reasons"]
        );
    }
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{target} {flag}");
    }
    check_aggregate_documents(&report);
}
