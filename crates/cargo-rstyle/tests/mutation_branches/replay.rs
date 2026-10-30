use super::*;

#[test]
fn branch_counts_and_thresholds_replay_from_full_or_compact_without_source_or_tools() {
    let directory = workspace(BRANCH);
    let (output, original) = requested(directory.path(), &["--report-detail", "compact"]);
    assert!(output.status.success());
    for path in ["src/lib.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let output = replay(directory.path(), &original, detail);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["verification_policy"], "strict-replay");
        assert_eq!(report["mutation_analysis"], original["mutation_analysis"]);
        assert_eq!(report["matching"], original["matching"]);
    }
    policy(directory.path(), 1);
    for detail in ["full", "compact"] {
        let output = replay(directory.path(), &original, detail);
        assert_eq!(output.status.code(), Some(1));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let original_function = function(&original, "branch");
        let changed = function(&report, "branch");
        assert_eq!(changed["evidence"], original_function["evidence"]);
        assert_eq!(rule(changed, "mutation_per_target")["actual"], 2);
        assert_eq!(rule(changed, "mutation_per_target")["limit"], 1);
        assert_eq!(rule(changed, "mutation_per_target")["outcome"], "finding");
    }
}
