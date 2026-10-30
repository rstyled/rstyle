use super::*;

#[test]
fn density_reviews_preserve_raw_findings_in_source_and_tool_free_full_compact_and_text_replay() {
    let directory = workspace(ORIGINAL);
    let (output, original) = capture(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    counts(&original, 0, 1);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    for path in ["src/lib.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    let hidden = replay.dismiss();
    counts(&hidden, 1, 0);
    raw_same(&hidden, &original);
    assert_eq!(hidden["mutation_effects"]["review_hidden_findings"], 1);
    assert_eq!(hidden["mutation_effects"]["visible_findings"], 0);
    let bytes = retained(directory.path());
    for detail in ["full", "compact"] {
        let output = replay.run(&["--fail-on-findings", "--report-detail", detail]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = parsed(&output);
        counts(&report, 1, 0);
        raw_same(&report, &original);
        assert_eq!(report["verification_policy"], "strict-replay");
    }
    let output =
        diagnostic_process::output(replay.command("text").args(["--fail-on-findings"])).unwrap();
    assert!(output.status.success());
    assert!(!stdout(&output).contains("\nside_effect_density many at"));
    assert_eq!(retained(directory.path()), bytes);
    let store: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(store["schema_version"], 7);
    assert_eq!(store["density_entries"].as_array().unwrap().len(), 1);
    assert_eq!(store["density_entries"][0]["reason"], REASON);
}

#[test]
fn density_dismissal_requires_a_saved_visible_completed_selected_finding() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--effect-density",
            "--review-file",
            "reviews.json",
            "--dismiss-density",
            key(&original),
            "--review-reason",
            REASON,
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--replay-run"));
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    for extra in [
        vec![
            "--dismiss-density",
            "missing-key",
            "--review-reason",
            REASON,
        ],
        vec!["--dismiss-density", key(&original), "--review-reason", " "],
        vec![
            "--max-analysis-functions",
            "0",
            "--dismiss-density",
            key(&original),
            "--review-reason",
            REASON,
        ],
    ] {
        replay.refused(&extra);
    }
    policy(directory.path(), &format!("exclude=['src/**']\n{POLICY}"));
    replay.refused(&[
        "--dismiss-density",
        key(&original),
        "--review-reason",
        REASON,
    ]);
}

#[test]
fn clean_unknown_and_source_suppressed_density_findings_cannot_be_dismissed() {
    for source in [
        CLEAN.to_owned(),
        UNKNOWN.to_owned(),
        format!("// rstyle-allow(side_effect_density): deliberate\n{ORIGINAL}"),
    ] {
        let directory = workspace(&source);
        let (_, original) = capture(directory.path(), &[]);
        let replay = Replay {
            directory: directory.path(),
            original: &original,
        };
        let output = replay.run(&[
            "--dismiss-density",
            key(&original),
            "--review-reason",
            REASON,
        ]);
        assert_eq!(output.status.code(), Some(2));
        assert!(!directory.path().join("reviews.json").exists());
    }
}

#[test]
fn changed_density_threshold_weights_and_path_policy_never_hide_or_prune_reviews() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss();
    let bytes = retained(directory.path());
    for changed in [
        POLICY.replace("max_density_milli=100", "max_density_milli=200"),
        format!("{POLICY}\n[side_effects.weights]\nmutation=2\n"),
        format!("{POLICY}\n[side_effects.paths]\ntime=['std::thread::yield_now']\n"),
    ] {
        policy(directory.path(), &changed);
        let output = replay.run(&["--review-prune"]);
        let report = parsed(&output);
        counts(&report, 0, 1);
        assert_eq!(report["review_status"]["records"][0]["state"], "stale");
        assert_eq!(report["review_prune"]["eligible_entries"], 0);
        assert_eq!(retained(directory.path()), bytes);
    }
}

#[test]
fn changed_event_distribution_and_statement_denominator_invalidate_density_reviews() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss();
    let bytes = retained(directory.path());
    for source in [
        "pub fn many(target:&mut u32,other:&mut u32){*target=1;*other=2;}\n",
        "pub fn many(target:&mut u32){let value=0u32;*target=value;}\n",
    ] {
        std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
        let (_, report) = capture(directory.path(), &["--review-status"]);
        counts(&report, 0, 1);
        assert_eq!(report["review_status"]["records"][0]["state"], "stale");
        assert_eq!(retained(directory.path()), bytes);
    }
}
