use super::*;

#[test]
fn same_policy_completed_negative_density_reviews_are_pruned_with_a_preview() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss();
    let bytes = retained(directory.path());
    std::fs::write(directory.path().join("src/lib.rs"), CLEAN).unwrap();
    let (_, negative) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &negative,
    };
    let output = replay.run(&["--review-prune", "--review-prune-dry-run"]);
    let report = parsed(&output);
    assert!(output.status.success());
    assert_eq!(report["review_status"]["records"][0]["state"], "no_finding");
    assert_eq!(report["review_prune"]["eligible_entries"], 1);
    assert_eq!(report["review_prune"]["removed_entries"], 0);
    assert_eq!(
        report["review_prune"]["selections"][0],
        serde_json::json!({"family":"side_effect_density","entry_index":0})
    );
    assert_eq!(retained(directory.path()), bytes);
    let output = replay.run(&["--review-prune"]);
    assert!(output.status.success());
    assert_eq!(parsed(&output)["review_prune"]["removed_entries"], 1);
    let applied = retained(directory.path());
    assert_eq!(
        parsed(&replay.run(&["--review-prune"]))["review_prune"]["removed_entries"],
        0
    );
    assert_eq!(retained(directory.path()), applied);
}

#[test]
fn omitted_unselected_unknown_and_budgeted_density_reviews_are_retained() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss();
    let bytes = retained(directory.path());
    std::fs::write(
        directory.path().join("selection.json"),
        "{\"schema\":\"rstyle.changed-files/1\",\"paths\":[]}",
    )
    .unwrap();
    for extra in [
        vec!["--max-analysis-functions", "0"],
        vec!["--source-rules", "--changed-files", "selection.json"],
    ] {
        let mut args = vec!["--review-prune"];
        args.extend(extra);
        let report = parsed(&replay.run(&args));
        assert_eq!(report["density_review"]["hidden_findings"], 0);
        assert_eq!(report["review_prune"]["eligible_entries"], 0);
        assert_eq!(retained(directory.path()), bytes);
    }
    assert_eq!(
        replay
            .run(&[
                "--source-rules",
                "--changed-files",
                "selection.json",
                "--dismiss-density",
                key(&original),
                "--review-reason",
                REASON,
            ])
            .status
            .code(),
        Some(2)
    );
    let report = replay.omitted();
    assert_eq!(
        report["review_status"]["records"][0]["state"],
        "not_requested"
    );
    assert_eq!(report["review_prune"]["eligible_entries"], 0);
    assert_unknown_retained(directory.path(), &bytes);
}

fn assert_unknown_retained(directory: &Path, bytes: &[u8]) {
    std::fs::write(directory.join("src/lib.rs"), UNKNOWN).unwrap();
    let (_, unknown) = capture(directory, &[]);
    let report = parsed(
        &Replay {
            directory,
            original: &unknown,
        }
        .run(&["--review-prune"]),
    );
    assert_eq!(report["density_review"]["hidden_findings"], 0);
    assert_eq!(report["review_prune"]["eligible_entries"], 0);
    assert_eq!(retained(directory), bytes);
}
