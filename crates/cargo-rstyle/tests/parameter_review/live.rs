use super::*;

#[test]
fn missing_store_lookup_never_writes_and_only_saved_run_dismissal_records_minimal_signature() {
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
            "--dismiss-parameter",
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
    assert_saved_parameter(&store);
}

fn assert_saved_parameter(store: &Value) {
    assert_eq!(store["schema_version"], 3);
    assert_eq!(store["parameter_entries"].as_array().unwrap().len(), 1);
    let entry = &store["parameter_entries"][0];
    assert_eq!(entry["reason"], REASON);
    assert_eq!(
        entry["finding"]["policy"],
        serde_json::json!({"max_params":1,"count_self":false})
    );
    assert_eq!(
        entry["finding"]["signature"],
        serde_json::json!({"parameters":2,"has_self":false,"actual":2})
    );
    let fields = entry["finding"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        fields,
        ["context", "locator", "policy", "signature", "versions"]
    );
    assert_eq!(entry["finding"]["locator"]["source"], "src/lib.rs");
}

#[test]
fn hidden_parameters_replay_without_original_files_or_tools_in_full_compact_and_text() {
    let directory = workspace(ORIGINAL);
    let (output, original) = capture(directory.path(), &[]);
    assert!(output.status.success());
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
    let mut reports = Vec::new();
    for detail in ["full", "compact"] {
        let output = replay.run(&[
            "--fail-on-findings",
            "--report-detail",
            detail,
            "--parameter-plans",
        ]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = parsed(&output);
        assert_hidden_review(&report);
        assert_eq!(report["analysis"]["findings"], 1);
        assert_eq!(report["parameter_planning"]["findings"], 1);
        assert_eq!(report["verification_policy"], "strict-replay");
        reports.push(report["parameter_review"].clone());
    }
    assert_eq!(reports[0], reports[1]);
    let output = diagnostic_process::output(replay.command("text").args([
        "--source-rules",
        "--analysis-config",
        "analysis.toml",
        "--fail-on-findings",
    ]))
    .unwrap();
    assert!(output.status.success());
    assert!(!stdout(&output).contains("\nfn_params "));
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

fn assert_hidden_review(report: &Value) {
    counts(report, 1, 0);
    assert_eq!(report["parameter_review"]["coverage"], "complete");
    assert_eq!(
        report["parameter_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(report["parameter_review"]["decisions"][0]["selected"], true);
}

#[test]
fn layout_binder_and_body_changes_preserve_the_reviewed_parameter_boundary() {
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
        "// shifted original declaration\n\npub fn many(left: u32, right: u32) -> u32 {\n    let saved = left * right;\n    saved + 3\n}\n").unwrap();
    let (output, changed) = capture(directory.path(), &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", changed["parameter_review"]);
    counts(&changed, 1, 0);
    assert_eq!(
        changed["parameter_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn arity_threshold_and_count_self_policy_changes_make_positive_reviews_stale() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("many");
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    for (source, policy) in [
        (
            "pub fn many(a: u32, b: u32, c: u32) -> u32 { a + b + c }\n",
            "max_params=1\ncount_self=false",
        ),
        (ORIGINAL, "max_params=0\ncount_self=false"),
        (ORIGINAL, "max_params=1\ncount_self=true"),
    ] {
        std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
        std::fs::write(
            directory.path().join("analysis.toml"),
            format!("[fn_params]\n{policy}\n"),
        )
        .unwrap();
        let (output, changed) =
            capture(directory.path(), &["--fail-on-findings", "--review-status"]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            changed["parameter_review"]
        );
        counts(&changed, 0, 1);
        assert_eq!(
            changed["parameter_review"]["decisions"][0]["state"],
            "stale"
        );
        assert_eq!(changed["review_status"]["records"][0]["state"], "stale");
        assert_eq!(std::fs::read(&file).unwrap(), bytes);
    }
}

#[test]
fn hidden_parameters_do_not_hide_length_or_guard_findings() {
    let directory = workspace(
        "pub fn many(a: u32, b: u32) -> u32 {\n    if a > 0 {\n        let next = a + b;\n        next + 1\n    } else { 0 }\n}\n",
    );
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=2\n[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    let (_, original) = capture(directory.path(), &["--guard-clauses"]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("many");
    let output = Replay {
        directory: directory.path(),
        original: &original,
    }
    .run(&["--guard-clauses", "--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    let report = parsed(&output);
    counts(&report, 1, 0);
    assert_eq!(report["analysis"]["findings"], 3);
    assert_eq!(
        report["analysis"]["functions"][0]["guard_clause"]["outcome"],
        "finding"
    );
}

#[test]
fn suppressed_clean_unselected_and_unknown_selectors_cannot_change_the_store() {
    let directory = workspace(
        "pub fn many(a: u32, b: u32) -> u32 { a + b }\n\
        pub fn clean(value: u32) -> u32 { value }\n\
        // rstyle-allow(fn_params): generated boundary\n\
        pub fn allowed(a: u32, b: u32) -> u32 { a + b }\n",
    );
    let (_, original) = capture(directory.path(), &[]);
    let file = directory.path().join("reviews.json");
    let bytes = b"{\"schema_version\":2,\"entries\":[],\"source_entries\":[]}\n";
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
        let output = replay.run(&["--dismiss-parameter", selector, "--review-reason", REASON]);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(std::fs::read(&file).unwrap(), bytes);
    }
    std::fs::write(
        directory.path().join("selection.json"),
        "{\"schema\":\"rstyle.changed-files/1\",\"paths\":[]}",
    )
    .unwrap();
    let output = replay.run(&[
        "--dismiss-parameter",
        key(&original, "many"),
        "--review-reason",
        REASON,
        "--changed-files",
        "selection.json",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn budget_limits_remain_incomplete() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss("many");
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    let output = replay.run(&["--max-analysis-functions", "0", "--review-status"]);
    assert_eq!(output.status.code(), Some(2));
    let limited = parsed(&output);
    assert_eq!(limited["parameter_review"]["coverage"], "incomplete");
    assert_eq!(
        limited["review_status"]["records"][0]["state"],
        "unavailable"
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn ordinary_external_module_parameter_reviews_keep_original_identity_and_raw_findings() {
    let external = workspace("pub mod external;\n");
    std::fs::write(external.path().join("src/external.rs"), ORIGINAL).unwrap();
    let (output, report) = capture(external.path(), &[]);
    assert!(output.status.success());
    assert_eq!(report["parameter_review"]["coverage"], "complete");
    counts(&report, 0, 1);
    let replay = Replay {
        directory: external.path(),
        original: &report,
    };
    replay.dismiss("external::many");

    let store: Value =
        serde_json::from_slice(&std::fs::read(external.path().join("reviews.json")).unwrap())
            .unwrap();
    let locator = &store["parameter_entries"][0]["finding"]["locator"];
    assert_eq!(locator["source"], "src/external.rs");
    assert_eq!(locator["qualified_name"], "crate::external::many");
    let (output, reviewed) = capture(external.path(), &["--fail-on-findings"]);
    assert!(output.status.success());
    counts(&reviewed, 1, 0);
    assert_eq!(reviewed["analysis"]["findings"], 1);
}

#[test]
fn remapped_external_module_parameter_identity_remains_unavailable() {
    let external = workspace("#[path=\"external.rs\"] pub mod external;\n");
    std::fs::write(external.path().join("src/external.rs"), ORIGINAL).unwrap();
    let (output, report) = capture(external.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["analysis"]["findings"], 1);
    assert_eq!(
        report["parameter_review"]["decisions"][0]["state"],
        "unavailable"
    );
    let output = Replay {
        directory: external.path(),
        original: &report,
    }
    .run(&[
        "--dismiss-parameter",
        key(&report, "external::many"),
        "--review-reason",
        REASON,
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!external.path().join("reviews.json").exists());
}
