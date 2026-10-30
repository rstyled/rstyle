use super::*;

#[test]
fn full_compact_and_text_replay_keep_original_mutation_comments_after_live_edits() {
    let directory = workspace(&format!(
        "// rstyle-allow(mutation_per_target, mutated_targets): reviewed local state\n{WRITES}"
    ));
    let (output, original) = requested(directory.path(), &["--fail-on-findings"]);
    assert!(output.status.success());
    std::fs::write(directory.path().join("src/lib.rs"), WRITES).unwrap();
    for detail in ["full", "compact"] {
        let (output, report) = replay(directory.path(), &original, detail);
        assert!(output.status.success());
        assert_eq!(report["verification_policy"], "strict-replay");
        assert_eq!(report["mutation_analysis"], original["mutation_analysis"]);
    }
    for path in ["src/lib.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let (output, report) = replay(directory.path(), &original, detail);
        assert!(output.status.success());
        counts(&report, 2, 0, 2);
        assert_eq!(report["mutation_analysis"], original["mutation_analysis"]);
    }
    let output =
        diagnostic_process::output(&mut replay_command(directory.path(), &original, "text"))
            .unwrap();
    assert!(output.status.success());
    assert!(!stdout(&output).contains("\nmutation_per_target "));
    assert!(!stdout(&output).contains("\nmutated_targets "));
}

#[test]
fn replay_exclusions_use_original_paths_and_preserve_opaque_raw_evidence_without_tools() {
    let directory = workspace(OPAQUE);
    let (_, original) = requested(directory.path(), &[]);
    for path in ["src/lib.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    policy(directory.path(), &["src/lib.rs"]);
    for detail in ["full", "compact"] {
        let (output, report) = replay(directory.path(), &original, detail);
        assert!(output.status.success(), "{}", report["mutation_analysis"]);
        assert_raw_same(function(&report), function(&original));
        assert_eq!(function(&report)["coverage"], "incomplete");
        assert_eq!(function(&report)["policy"]["coverage"], "complete");
        assert_eq!(report["mutation_analysis"]["excluded_functions"], 1);
        assert_eq!(
            report["mutation_analysis"]["policy_unavailable_functions"],
            0
        );
    }
    policy(directory.path(), &[]);
    let (output, report) = replay(directory.path(), &original, "compact");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(function(&report)["policy"]["coverage"], "incomplete");
}
