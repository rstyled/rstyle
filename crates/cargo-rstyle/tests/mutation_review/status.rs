use super::*;

#[test]
fn older_review_schemas_are_read_without_upgrading_or_rewriting() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    for version in 1..=5 {
        let bytes = format!("{{\"schema_version\":{version},\"entries\":[]}}\n");
        std::fs::write(directory.path().join("reviews.json"), &bytes).unwrap();
        let output = replay.run(&["--review-status", "--fail-on-findings"]);
        assert_eq!(output.status.code(), Some(1));
        let report = parsed(&output);
        counts(&report, 0, 2);
        assert!(
            report["review_status"]["records"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(retained(directory.path()), bytes.as_bytes());
    }
}

#[test]
fn changed_mutation_thresholds_are_stale_and_never_negative_pruning_authority() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss("--dismiss-mutation-per-target");
    let bytes = retained(directory.path());
    for maximum in [1, 100] {
        policy(directory.path(), maximum);
        let output = replay.run(&["--review-prune", "--review-prune-dry-run"]);
        let report = parsed(&output);
        assert_eq!(report["review_status"]["records"][0]["state"], "stale");
        assert_eq!(report["review_prune"]["eligible_entries"], 0);
        assert_eq!(report["review_prune"]["removed_entries"], 0);
        let output = replay.run(&["--review-prune"]);
        assert_eq!(parsed(&output)["review_prune"]["removed_entries"], 0);
        assert_eq!(retained(directory.path()), bytes);
    }
}

#[test]
fn completed_negative_mutation_review_pruning_is_previewable_and_idempotent() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("--dismiss-mutation-per-target");
    let bytes = retained(directory.path());
    std::fs::write(directory.path().join("src/lib.rs"), CLEAN).unwrap();
    let (_, negative) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &negative,
    };
    let output = replay.run(&["--review-prune", "--review-prune-dry-run"]);
    assert!(output.status.success());
    let report = parsed(&output);
    assert_eq!(report["review_status"]["records"][0]["state"], "no_finding");
    assert_eq!(report["review_prune"]["eligible_entries"], 1);
    assert_eq!(report["review_prune"]["removed_entries"], 0);
    assert_eq!(
        report["review_prune"]["selections"][0],
        serde_json::json!({"family":"mutation_per_target","entry_index":0})
    );
    assert_eq!(retained(directory.path()), bytes);
    let output = replay.run(&["--review-prune"]);
    assert!(output.status.success());
    assert_eq!(parsed(&output)["review_prune"]["removed_entries"], 1);
    let applied = retained(directory.path());
    let output = replay.run(&["--review-prune"]);
    assert!(output.status.success());
    assert_eq!(parsed(&output)["review_prune"]["removed_entries"], 0);
    assert_eq!(retained(directory.path()), applied);
}

#[test]
fn omitted_mutation_analysis_and_missing_declarations_are_distinct_and_never_deleted() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("--dismiss-mutation-per-target");
    let bytes = retained(directory.path());
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    let output = diagnostic_process::output(command.current_dir(directory.path()).args([
        "clones",
        "--format",
        "json",
        "--source-rules",
        "--extraction-profile",
        "hir-data-v3",
        "--analysis-config",
        "analysis.toml",
        "--review-file",
        "reviews.json",
        "--review-status",
        "--replay-run",
        original["run_directory"].as_str().unwrap(),
    ]))
    .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let report = parsed(&output);
    assert_eq!(report["mutation_review"]["execution"], "not_requested");
    assert_eq!(report["mutation_review"]["hidden_findings"], 0);
    assert_eq!(
        report["review_status"]["records"][0]["state"],
        "not_requested"
    );
    std::fs::write(
        directory.path().join("src/lib.rs"),
        ORIGINAL.replace("fn many", "fn other"),
    )
    .unwrap();
    let (output, report) = capture(directory.path(), &["--review-status"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        report["review_status"]["records"][0]["state"],
        "not_observed"
    );
    assert_eq!(retained(directory.path()), bytes);
}

#[test]
fn unknown_remainders_and_execution_budgets_never_hide_or_prune_retained_reviews() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("--dismiss-mutation-per-target");
    let bytes = retained(directory.path());
    std::fs::write(directory.path().join("src/lib.rs"), OPAQUE).unwrap();
    let (_, unknown) = capture(directory.path(), &[]);
    for (report, extra) in [
        (&unknown, vec![]),
        (&original, vec!["--max-analysis-functions", "0"]),
    ] {
        let replay = Replay {
            directory: directory.path(),
            original: report,
        };
        let mut args = vec!["--review-prune", "--fail-on-findings"];
        args.extend(extra);
        let output = replay.run(&args);
        assert_eq!(output.status.code(), Some(2));
        let report = parsed(&output);
        assert_eq!(report["mutation_review"]["hidden_findings"], 0);
        assert_eq!(report["mutation_review"]["coverage"], "incomplete");
        assert_eq!(report["review_prune"]["eligible_entries"], 0);
        assert_eq!(retained(directory.path()), bytes);
    }
}

#[test]
fn changed_file_selection_never_hides_dismisses_or_prunes_an_unselected_mutation() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss("--dismiss-mutation-per-target");
    let bytes = retained(directory.path());
    std::fs::write(
        directory.path().join("selection.json"),
        "{\"schema\":\"rstyle.changed-files/1\",\"paths\":[]}",
    )
    .unwrap();
    let flags = [
        "--source-rules",
        "--analysis-config",
        "analysis.toml",
        "--changed-files",
        "selection.json",
    ];
    let mut args = flags.to_vec();
    args.extend(["--review-prune", "--fail-on-findings"]);
    let output = replay.run(&args);
    let report = parsed(&output);
    assert_eq!(report["mutation_review"]["hidden_findings"], 0);
    assert_eq!(report["mutation_review"]["hidden_selected_findings"], 0);
    assert_eq!(report["mutation_review"]["decisions"][0]["selected"], false);
    assert_eq!(report["review_prune"]["eligible_entries"], 0);
    let mut args = flags.to_vec();
    args.extend([
        "--dismiss-mutated-targets",
        key(&original),
        "--review-reason",
        REASON,
    ]);
    assert_eq!(replay.run(&args).status.code(), Some(2));
    assert_eq!(retained(directory.path()), bytes);
}
