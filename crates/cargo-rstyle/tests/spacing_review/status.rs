use super::*;

#[test]
fn old_schema_lookup_and_status_do_not_upgrade_or_rewrite_retained_bytes() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    for version in [1, 2, 3, 4] {
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
fn omitted_spacing_policy_is_not_requested_and_missing_declarations_are_not_deleted() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss("many");
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    policy(directory.path(), None);
    let output = replay.run(&["--review-status"]);
    assert_eq!(output.status.code(), Some(2));
    let omitted = parsed(&output);
    assert!(omitted.get("spacing_review").is_none());
    assert_eq!(omitted["review_status"]["records"][0]["family"], "spacing");
    assert_eq!(
        omitted["review_status"]["records"][0]["state"],
        "not_requested"
    );
    policy(directory.path(), Some(5));
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
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn completed_negative_spacing_pruning_is_explicit_previewable_and_idempotent() {
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
        ORIGINAL.replace("    let third", " \t\n    let third"),
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
    assert_eq!(report["review_status"]["records"][0]["state"], "no_finding");
    assert_eq!(report["review_prune"]["eligible_entries"], 1);
    assert_eq!(report["review_prune"]["removed_entries"], 0);
    assert_eq!(
        report["review_prune"]["selections"][0],
        serde_json::json!({"family":"spacing","entry_index":0})
    );
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    let output = replay.run(&["--review-prune"]);
    assert!(output.status.success());
    assert_eq!(parsed(&output)["review_prune"]["removed_entries"], 1);
    let applied = std::fs::read(&file).unwrap();
    let store: Value = serde_json::from_slice(&applied).unwrap();
    assert_eq!(store["schema_version"], 5);
    assert!(
        store
            .get("spacing_entries")
            .is_none_or(|entries| entries.as_array().is_some_and(Vec::is_empty))
    );
    let output = replay.run(&["--review-prune"]);
    assert!(output.status.success());
    assert_eq!(parsed(&output)["review_prune"]["removed_entries"], 0);
    assert_eq!(std::fs::read(file).unwrap(), applied);
}

#[test]
fn library_and_test_compilation_contexts_do_not_share_a_spacing_dismissal() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &["--all-targets"]);
    let output = Replay {
        directory: directory.path(),
        original: &original,
    }
    .run(&[
        "--dismiss-spacing",
        key(&original, "many"),
        "--review-reason",
        REASON,
    ]);
    let report = parsed(&output);
    counts(&report, 1, 1);
    assert_eq!(report["spacing_review"]["coverage"], "complete");
    assert!(
        report["analysis"]["scope"]["outside_source_functions"]
            .as_u64()
            .unwrap()
            > 0
    );
    let store: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("reviews.json")).unwrap())
            .unwrap();
    assert_eq!(store["spacing_entries"].as_array().unwrap().len(), 1);
}

#[test]
fn raised_or_disabled_spacing_policy_never_authorizes_negative_pruning() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss("many");
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    for minimum in [100, 0] {
        policy(directory.path(), Some(minimum));
        for flags in [
            vec!["--review-prune", "--review-prune-dry-run"],
            vec!["--review-prune"],
        ] {
            let output = replay.run(&flags);
            let report = parsed(&output);
            assert!(output.status.success(), "{}", report["review_status"]);
            counts(&report, 0, 0);
            assert_eq!(report["review_status"]["records"][0]["state"], "stale");
            assert_eq!(report["review_prune"]["eligible_entries"], 0);
            assert_eq!(report["review_prune"]["removed_entries"], 0);
            assert_eq!(std::fs::read(&file).unwrap(), bytes);
        }
    }
}

#[test]
fn an_empty_capture_distinguishes_requested_spacing_from_an_omitted_policy() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("many");
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), "").unwrap();
    let (output, empty) = capture(directory.path(), &["--review-status"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        empty["analysis"]["functions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    counts(&empty, 0, 0);
    assert_eq!(empty["spacing_review"]["coverage"], "complete");
    assert_eq!(
        empty["review_status"]["records"][0]["state"],
        "not_observed"
    );
    policy(directory.path(), None);
    let output = Replay {
        directory: directory.path(),
        original: &empty,
    }
    .run(&["--review-status"]);
    assert_eq!(output.status.code(), Some(2));
    let omitted = parsed(&output);
    assert!(omitted.get("spacing_review").is_none());
    assert_eq!(
        omitted["review_status"]["records"][0]["state"],
        "not_requested"
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}
