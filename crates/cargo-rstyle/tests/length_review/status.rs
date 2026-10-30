use super::*;

#[test]
fn old_schema_lookup_and_status_do_not_upgrade_or_rewrite_retained_bytes() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    for version in [1, 2, 3] {
        let bytes = format!("{{\"schema_version\":{version},\"entries\":[]}}\n");
        let file = directory.path().join("reviews.json");
        std::fs::write(&file, &bytes).unwrap();
        let output = replay.run(&["--review-status", "--fail-on-findings"]);
        assert_eq!(output.status.code(), Some(1));
        let report = parsed(&output);
        counts(&report, 0, 1);
        assert!(
            report["review_status"]["records"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(std::fs::read_to_string(file).unwrap(), bytes);
    }
}

#[test]
fn retained_length_status_distinguishes_omitted_negative_and_missing_declarations() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss("many");
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    let output = diagnostic_process::output(
        replay
            .command("json")
            .args(["--helper-proposals", "--review-status"]),
    )
    .unwrap();
    let omitted = parsed(&output);
    assert!(omitted.get("length_review").is_none());
    assert_eq!(omitted["review_status"]["records"][0]["family"], "length");
    assert_eq!(
        omitted["review_status"]["records"][0]["state"],
        "not_requested"
    );
    for (source, state) in [
        ("pub fn many(a: u32) -> u32 { a }\n", "no_finding"),
        ("pub fn other(a: u32) -> u32 { a }\n", "not_observed"),
    ] {
        std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
        let (output, report) = capture(directory.path(), &["--review-status"]);
        assert_eq!(
            output.status.code(),
            Some(if state == "not_observed" { 2 } else { 0 }),
            "{}",
            report["review_status"]
        );
        assert_eq!(report["review_status"]["records"][0]["state"], state);
        assert_eq!(std::fs::read(&file).unwrap(), bytes);
    }
}

#[test]
fn completed_negative_length_pruning_is_explicit_previewable_and_idempotent() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("many");
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn many(a: u32) -> u32 { a }\n",
    )
    .unwrap();
    let (_, negative) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &negative,
    };
    let preview = replay.run(&["--review-prune", "--review-prune-dry-run"]);
    assert!(preview.status.success());
    let report = parsed(&preview);
    assert_eq!(report["review_prune"]["eligible_entries"], 1);
    assert_eq!(report["review_prune"]["removed_entries"], 0);
    assert_eq!(
        report["review_prune"]["selections"][0],
        serde_json::json!({"family":"length","entry_index":0})
    );
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    let output = replay.run(&["--review-prune"]);
    assert!(output.status.success());
    assert_eq!(parsed(&output)["review_prune"]["removed_entries"], 1);
    let applied = std::fs::read(&file).unwrap();
    let store: Value = serde_json::from_slice(&applied).unwrap();
    assert_eq!(store["schema_version"], 4);
    assert!(
        store
            .get("length_entries")
            .is_none_or(|entries| entries.as_array().is_some_and(Vec::is_empty))
    );
    let output = replay.run(&["--review-prune"]);
    assert!(output.status.success());
    assert_eq!(parsed(&output)["review_prune"]["removed_entries"], 0);
    assert_eq!(std::fs::read(file).unwrap(), applied);
}

#[test]
fn library_and_test_compilation_contexts_do_not_share_a_length_dismissal() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &["--all-targets"]);
    let output = Replay {
        directory: directory.path(),
        original: &original,
    }
    .run(&[
        "--dismiss-length",
        key(&original, "many"),
        "--review-reason",
        REASON,
    ]);
    let report = parsed(&output);
    counts(&report, 1, 1);
    assert_eq!(report["length_review"]["coverage"], "complete");
    assert!(
        report["analysis"]["scope"]["outside_source_functions"]
            .as_u64()
            .unwrap()
            > 0
    );
    let store: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("reviews.json")).unwrap())
            .unwrap();
    assert_eq!(store["length_entries"].as_array().unwrap().len(), 1);
}

#[test]
fn raised_length_limit_does_not_turn_a_stale_review_into_a_prunable_negative() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss("many");
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=100\n[fn_params]\nmax_params=100\n",
    )
    .unwrap();
    for flags in [
        vec![
            "--review-status",
            "--review-prune",
            "--review-prune-dry-run",
        ],
        vec!["--review-status", "--review-prune"],
    ] {
        let output = replay.run(&flags);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = parsed(&output);
        counts(&report, 0, 0);
        assert_eq!(report["review_status"]["records"][0]["state"], "stale");
        assert_eq!(report["review_prune"]["eligible_entries"], 0);
        assert_eq!(report["review_prune"]["removed_entries"], 0);
        assert_eq!(std::fs::read(&file).unwrap(), bytes);
    }
}
