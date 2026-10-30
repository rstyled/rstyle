use super::*;

#[test]
fn full_and_compact_replay_reconfigure_static_loop_counts_without_source_or_tools() {
    let source = format!(
        "{FOR_LOOP}\npub fn continued(target:&mut u32,flag:bool) {{ while flag {{ if flag {{ continue; }} *target=1; }} }}\n"
    );
    let directory = workspace(&source);
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
    policy(directory.path(), 0);
    for detail in ["full", "compact"] {
        let output = replay(directory.path(), &original, detail);
        assert_eq!(output.status.code(), Some(1));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let changed = function(&report, "repeated");
        assert_eq!(
            changed["evidence"],
            function(&original, "repeated")["evidence"]
        );
        assert_eq!(rule(changed, "mutation_per_target")["actual"], 1);
        assert_eq!(rule(changed, "mutation_per_target")["limit"], 0);
        assert_eq!(rule(changed, "mutation_per_target")["outcome"], "finding");
    }
}
