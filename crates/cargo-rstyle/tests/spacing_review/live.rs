use super::*;

#[test]
fn spacing_dismissal_requires_a_saved_run_and_records_only_measurement_identity() {
    let directory = workspace(ORIGINAL);
    let (output, original) = capture(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    counts(&original, 0, 1);
    let file = directory.path().join("reviews.json");
    assert!(!file.exists());
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--source-rules",
            "--review-file",
            "reviews.json",
            "--dismiss-spacing",
            key(&original, "many"),
            "--review-reason",
            REASON,
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--replay-run"));
    assert!(!file.exists());
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("many");
    let store: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert_saved_spacing(&store);
}

fn assert_saved_spacing(store: &Value) {
    assert_eq!(store["schema_version"], 5);
    assert_eq!(store["spacing_entries"].as_array().unwrap().len(), 1);
    let entry = &store["spacing_entries"][0];
    assert_eq!(entry["reason"], REASON);
    assert_eq!(
        entry["finding"]["policy"],
        serde_json::json!({"min_statements":5})
    );
    assert_eq!(
        entry["finding"]["measurement"],
        serde_json::json!({"direct_statements":5})
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
fn hidden_spacing_replays_without_source_or_tools_and_preserves_raw_counts() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss("many");
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
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
        counts(&report, 1, 0);
        assert_eq!(report["spacing_review"]["coverage"], "complete");
        assert_eq!(
            report["spacing_review"]["decisions"][0]["state"],
            "hidden_exact"
        );
        assert_eq!(report["spacing_review"]["decisions"][0]["selected"], true);
        assert_eq!(report["spacing_review"]["hidden_selected_findings"], 1);
        assert_eq!(report["analysis"]["findings"], 1);
        assert_eq!(report["analysis"]["visible_findings"], 1);
        assert_eq!(report["verification_policy"], "strict-replay");
        reviews.push(report["spacing_review"].clone());
    }
    assert_eq!(reviews[0], reviews[1]);
    let output = diagnostic_process::output(replay.command("text").args([
        "--source-rules",
        "--analysis-config",
        "analysis.toml",
        "--fail-on-findings",
    ]))
    .unwrap();
    assert!(output.status.success());
    assert!(!stdout(&output).contains("\nfn_spacing "));
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn same_statement_count_body_binder_and_comment_edits_preserve_spacing_review() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("many");
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    std::fs::write(directory.path().join("src/lib.rs"),
        "// shifted declaration\n\npub fn many(left: u32, right: u32) -> u32 {\n    // changed computation\n    let first = left * right;\n    let second = first + 2;\n    let third = second + 2;\n    let fourth = third + 2;\n    fourth + 9 // inline comment\n}\n").unwrap();
    let (output, report) = capture(directory.path(), &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", report["spacing_review"]);
    counts(&report, 1, 0);
    assert_eq!(
        report["spacing_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn increased_statement_count_or_positive_spacing_policy_is_stale() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("many");
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    let longer = ORIGINAL.replace(
        "    fourth + 1",
        "    let fifth = fourth + 1;\n    fifth + 1",
    );
    for (source, minimum) in [(longer.as_str(), 5), (ORIGINAL, 4)] {
        std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
        policy(directory.path(), Some(minimum));
        let (output, report) =
            capture(directory.path(), &["--review-status", "--fail-on-findings"]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            report["spacing_review"]
        );
        counts(&report, 0, 1);
        assert_eq!(report["spacing_review"]["decisions"][0]["state"], "stale");
        assert_eq!(report["review_status"]["records"][0]["state"], "stale");
        assert_eq!(std::fs::read(&file).unwrap(), bytes);
    }
}

#[test]
fn invalid_clean_suppressed_unselected_and_unavailable_spacing_dismissals_never_write() {
    let directory = workspace(&format!(
        "{ORIGINAL}\npub fn clean(value: u32) -> u32 {{ value }}\n// rstyle-allow(fn_spacing): generated body\npub fn allowed(a: u32) -> u32 {{\n    let first = a + 1;\n    let second = first + 1;\n    let third = second + 1;\n    let fourth = third + 1;\n    fourth + 1\n}}\n"
    ));
    let (_, original) = capture(directory.path(), &[]);
    let file = directory.path().join("reviews.json");
    let bytes = b"{\"schema_version\":3,\"entries\":[]}\n";
    std::fs::write(&file, bytes).unwrap();
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    for selector in [
        key(&original, "clean"),
        key(&original, "allowed"),
        "function:u999:f999",
    ] {
        let output = replay.run(&["--dismiss-spacing", selector, "--review-reason", REASON]);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(std::fs::read(&file).unwrap(), bytes);
    }
    std::fs::write(
        directory.path().join("selection.json"),
        "{\"schema\":\"rstyle.changed-files/1\",\"paths\":[]}",
    )
    .unwrap();
    for extra in [
        vec!["--changed-files", "selection.json"],
        vec!["--max-analysis-functions", "0"],
    ] {
        let mut args = vec![
            "--dismiss-spacing",
            key(&original, "many"),
            "--review-reason",
            REASON,
        ];
        args.extend(extra);
        let output = replay.run(&args);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(std::fs::read(&file).unwrap(), bytes);
    }
}

#[test]
fn ordinary_external_module_spacing_identity_supports_saved_run_dismissal() {
    let directory = workspace("pub mod external;\n");
    std::fs::write(directory.path().join("src/external.rs"), ORIGINAL).unwrap();
    let (output, original) = capture(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    counts(&original, 0, 1);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss("external::many");

    let output = replay.run(&["--fail-on-findings"]);
    assert!(output.status.success());
    let report = parsed(&output);
    counts(&report, 1, 0);
    assert_eq!(report["spacing_review"]["coverage"], "complete");
    assert_eq!(
        report["spacing_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(report["analysis"]["findings"], 1);
    assert_eq!(report["analysis"]["visible_findings"], 1);
    let store: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("reviews.json")).unwrap())
            .unwrap();
    let locator = &store["spacing_entries"][0]["finding"]["locator"];
    assert_eq!(locator["source"], "src/external.rs");
    assert_eq!(locator["qualified_name"], "crate::external::many");
}

#[test]
fn path_remapped_external_module_spacing_identity_remains_unavailable() {
    let directory = workspace("#[path=\"external.rs\"] pub mod remapped;\n");
    std::fs::write(directory.path().join("src/external.rs"), ORIGINAL).unwrap();
    let (output, original) = capture(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        original["spacing_review"]["decisions"][0]["state"],
        "unavailable"
    );
    let output = Replay {
        directory: directory.path(),
        original: &original,
    }
    .run(&[
        "--dismiss-spacing",
        key(&original, "remapped::many"),
        "--review-reason",
        REASON,
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!directory.path().join("reviews.json").exists());
}

#[test]
fn three_source_rule_reviews_hide_independently_without_filtering_raw_plans() {
    let directory = workspace(ORIGINAL);
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=2\n[fn_params]\nmax_params=1\n[fn_spacing]\nmin_statements=5\n",
    )
    .unwrap();
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    for (selector, expected_hidden) in [("--dismiss-parameter", 1), ("--dismiss-length", 2)] {
        let output = replay.run(&[selector, key(&original, "many"), "--review-reason", REASON]);
        assert!(output.status.success());
        let output = replay.run(&["--fail-on-findings"]);
        assert_eq!(output.status.code(), Some(1));
        let report = parsed(&output);
        counts(&report, 0, 1);
        assert_eq!(report["parameter_review"]["hidden_findings"], 1);
        assert_eq!(
            report["length_review"]["hidden_findings"],
            expected_hidden - 1
        );
    }
    let file = directory.path().join("reviews.json");
    let previous: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(previous["schema_version"], 4);
    replay.dismiss("many");
    let store: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(store["schema_version"], 5);
    assert_eq!(store["parameter_entries"], previous["parameter_entries"]);
    assert_eq!(store["length_entries"], previous["length_entries"]);
    let output = replay.run(&["--fail-on-findings", "--parameter-plans"]);
    let report = parsed(&output);
    assert!(output.status.success(), "{}", report["parameter_planning"]);
    counts(&report, 1, 0);
    assert_eq!(report["parameter_review"]["hidden_findings"], 1);
    assert_eq!(report["length_review"]["hidden_findings"], 1);
    assert_eq!(report["analysis"]["findings"], 3);
    assert_eq!(report["analysis"]["visible_findings"], 3);
    assert_eq!(report["parameter_planning"]["findings"], 1);
    assert_eq!(store["spacing_entries"].as_array().unwrap().len(), 1);
}

#[test]
fn macro_transformed_statement_evidence_cannot_prune_or_dismiss_a_spacing_review() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("many");
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    let changed = ORIGINAL.replace("    let first", "    println!(\"trace\");\n    let first");
    std::fs::write(directory.path().join("src/lib.rs"), changed).unwrap();
    let (output, report) = capture(directory.path(), &["--review-status"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        report["review_status"]["records"][0]["state"],
        "unavailable"
    );
    let replay = Replay {
        directory: directory.path(),
        original: &report,
    };
    let output = replay.run(&["--review-prune"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(parsed(&output)["review_prune"]["eligible_entries"], 0);
    let output = replay.run(&[
        "--dismiss-spacing",
        key(&report, "many"),
        "--review-reason",
        REASON,
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}
