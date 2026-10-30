use super::*;

#[test]
fn saved_mutation_reviews_require_explicit_saved_run_and_hide_one_rule_at_a_time() {
    let directory = workspace(ORIGINAL);
    let (output, original) = capture(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    counts(&original, 0, 2);
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--mutation-rules",
            "--review-file",
            "reviews.json",
            "--dismiss-mutation-per-target",
            key(&original),
            "--review-reason",
            REASON,
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--replay-run"));
    assert!(!directory.path().join("reviews.json").exists());
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    let hidden = replay.dismiss("--dismiss-mutation-per-target");
    counts(&hidden, 1, 1);
    assert_raw_same(&hidden, &original);
    assert_eq!(replay.run(&["--fail-on-findings"]).status.code(), Some(1));
    let hidden = replay.dismiss("--dismiss-mutated-targets");
    counts(&hidden, 2, 0);
    assert_raw_same(&hidden, &original);
    assert_eq!(hidden["mutation_analysis"]["review_hidden_findings"], 2);
    assert!(replay.run(&["--fail-on-findings"]).status.success());
}

#[test]
fn mutation_review_store_retains_numeric_distribution_without_graph_or_source_snapshots() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("--dismiss-mutation-per-target");
    let store: Value = serde_json::from_slice(&retained(directory.path())).unwrap();
    assert_eq!(store["schema_version"], 6);
    let entry = &store["mutation_entries"][0];
    assert_eq!(entry["reason"], REASON);
    assert_eq!(
        entry["finding"]["measurement"],
        serde_json::json!({"sites_per_root":[3,3]})
    );
    assert_eq!(
        entry["finding"]["policy"],
        serde_json::json!({"kind":"mutation_per_target","max_mutations_per_target":2})
    );
    let fields = entry["finding"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        fields,
        ["context", "locator", "measurement", "policy", "versions"]
    );
    assert_eq!(entry["finding"]["locator"]["source"], "src/lib.rs");
}

#[test]
fn full_compact_and_text_mutation_review_replay_need_neither_sources_nor_tools() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss("--dismiss-mutation-per-target");
    replay.dismiss("--dismiss-mutated-targets");
    let bytes = retained(directory.path());
    for path in ["src/lib.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    let mut reviews = Vec::new();
    for detail in ["full", "compact"] {
        let output = replay.run(&["--fail-on-findings", "--report-detail", detail]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = parsed(&output);
        counts(&report, 2, 0);
        assert_raw_same(&report, &original);
        assert_eq!(report["verification_policy"], "strict-replay");
        reviews.push(report["mutation_review"].clone());
    }
    assert_eq!(reviews[0], reviews[1]);
    let output =
        diagnostic_process::output(replay.command("text").args(["--fail-on-findings"])).unwrap();
    assert!(output.status.success());
    assert!(!stdout(&output).contains("\nmutation_per_target "));
    assert!(!stdout(&output).contains("\nmutated_targets "));
    assert_eq!(retained(directory.path()), bytes);
}

#[test]
fn binder_comment_and_rhs_edits_keep_reviews_but_changed_root_distribution_is_stale() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("--dismiss-mutation-per-target");
    let bytes = retained(directory.path());
    let renamed = format!(
        "// shifted declaration\n{}",
        ORIGINAL.replace("first", "renamed").replace("=2", "=9")
    );
    std::fs::write(directory.path().join("src/lib.rs"), renamed).unwrap();
    let (_, report) = capture(directory.path(), &[]);
    counts(&report, 1, 1);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        ORIGINAL.replace("second=3;", ""),
    )
    .unwrap();
    let (_, report) = capture(directory.path(), &["--review-status"]);
    counts(&report, 0, 2);
    assert_eq!(report["review_status"]["records"][0]["state"], "stale");
    assert_eq!(retained(directory.path()), bytes);
}

#[test]
fn clean_suppressed_excluded_unknown_and_budgeted_mutation_dismissals_never_write() {
    for source in [
        CLEAN.to_owned(),
        format!("// rstyle-allow(mutation_per_target): deliberate\n{ORIGINAL}"),
        OPAQUE.to_owned(),
    ] {
        let directory = workspace(&source);
        let (_, report) = capture(directory.path(), &[]);
        let output = Replay {
            directory: directory.path(),
            original: &report,
        }
        .run(&[
            "--dismiss-mutation-per-target",
            key(&report),
            "--review-reason",
            REASON,
        ]);
        assert_eq!(output.status.code(), Some(2));
        assert!(!directory.path().join("reviews.json").exists());
    }
    let directory = workspace(ORIGINAL);
    let (_, report) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &report,
    };
    let output = replay.run(&[
        "--max-analysis-functions",
        "0",
        "--dismiss-mutation-per-target",
        key(&report),
        "--review-reason",
        REASON,
    ]);
    assert_eq!(output.status.code(), Some(2));
    std::fs::write(directory.path().join("mutation.toml"), "exclude=[\"src/lib.rs\"]\n[mutation_budget]\nmax_mutations_per_target=2\nmax_mutated_targets=1\n").unwrap();
    let output = replay.run(&[
        "--dismiss-mutation-per-target",
        key(&report),
        "--review-reason",
        REASON,
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!directory.path().join("reviews.json").exists());
}

#[test]
fn adding_mutation_reviews_preserves_existing_source_rule_reviews() {
    let source = ORIGINAL
        .replace("many()", "many(a:u32,b:u32)")
        .replace(" first=1;", "\n first=a+b;\n");
    let directory = workspace(&source);
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=1\n[fn_params]\nmax_params=1\n[fn_spacing]\nmin_statements=1\n",
    )
    .unwrap();
    let source_flags = ["--source-rules", "--analysis-config", "analysis.toml"];
    let (_, original) = capture(directory.path(), &source_flags);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    for flag in [
        "--dismiss-parameter",
        "--dismiss-length",
        "--dismiss-spacing",
    ] {
        let mut args = source_flags.to_vec();
        args.extend([flag, key(&original), "--review-reason", REASON]);
        let output = replay.run(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let previous: Value = serde_json::from_slice(&retained(directory.path())).unwrap();
    assert_eq!(previous["schema_version"], 5);
    replay.dismiss("--dismiss-mutation-per-target");
    let store: Value = serde_json::from_slice(&retained(directory.path())).unwrap();
    assert_eq!(store["schema_version"], 6);
    for field in [
        "entries",
        "source_entries",
        "parameter_entries",
        "length_entries",
        "spacing_entries",
    ] {
        assert_eq!(store[field], previous[field]);
    }
    assert_eq!(store["mutation_entries"].as_array().unwrap().len(), 1);
}
