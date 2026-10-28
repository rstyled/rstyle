use super::*;

#[test]
fn length_dismissal_requires_a_saved_run_and_records_only_measurement_identity() {
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
            "--dismiss-length",
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
    assert_saved_length(&store);
}

fn assert_saved_length(store: &Value) {
    assert_eq!(store["schema_version"], 4);
    assert_eq!(store["length_entries"].as_array().unwrap().len(), 1);
    let entry = &store["length_entries"][0];
    assert_eq!(entry["reason"], REASON);
    assert_eq!(
        entry["finding"]["policy"],
        serde_json::json!({"max_lines":2})
    );
    assert_eq!(
        entry["finding"]["measurement"],
        serde_json::json!({"code_lines":4})
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
fn hidden_length_replays_without_source_or_tools_and_preserves_raw_counts() {
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
        assert_eq!(report["length_review"]["coverage"], "complete");
        assert_eq!(
            report["length_review"]["decisions"][0]["state"],
            "hidden_exact"
        );
        assert_eq!(report["length_review"]["decisions"][0]["selected"], true);
        assert_eq!(report["length_review"]["hidden_selected_findings"], 1);
        assert_eq!(report["analysis"]["findings"], 1);
        assert_eq!(report["analysis"]["visible_findings"], 1);
        assert_eq!(report["verification_policy"], "strict-replay");
        reviews.push(report["length_review"].clone());
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
    assert!(!stdout(&output).contains("\nfn_length "));
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn changed_body_comments_and_blank_lines_preserve_an_unchanged_code_line_measurement() {
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
        "// moved declaration\n\npub fn many(left: u32, right: u32) -> u32 {\n\n    // changed computation\n    let product = left * right;\n    product + 9 // inline comment\n}\n").unwrap();
    let (output, changed) = capture(directory.path(), &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", changed["length_review"]);
    counts(&changed, 1, 0);
    assert_eq!(
        changed["length_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn changed_measurement_or_positive_length_policy_is_stale() {
    let directory = workspace(ORIGINAL);
    let (_, original) = capture(directory.path(), &[]);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("many");
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    for (source, limit) in [
        (
            "pub fn many(a: u32, b: u32) -> u32 {\n    let sum = a + b;\n    let next = sum + 2;\n    next + 1\n}\n",
            2,
        ),
        (ORIGINAL, 1),
    ] {
        std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
        std::fs::write(
            directory.path().join("analysis.toml"),
            format!("[fn_length]\nmax_lines={limit}\n[fn_params]\nmax_params=100\n"),
        )
        .unwrap();
        let (output, report) =
            capture(directory.path(), &["--review-status", "--fail-on-findings"]);
        assert_eq!(output.status.code(), Some(1));
        counts(&report, 0, 1);
        assert_eq!(report["length_review"]["decisions"][0]["state"], "stale");
        assert_eq!(report["review_status"]["records"][0]["state"], "stale");
        assert_eq!(std::fs::read(&file).unwrap(), bytes);
    }
}

#[test]
fn length_and_parameter_reviews_are_independent_and_preserve_other_rules_and_plans() {
    let directory = workspace(
        "pub fn many(a: u32, b: u32) -> u32 {\n    let first = a + b;\n    let second = first + 1;\n    let third = second + 1;\n    let fourth = third + 1;\n    if a > 0 {\n        let next = fourth + b;\n        next + 1\n    } else { 0 }\n}\n",
    );
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=2\n[fn_params]\nmax_params=1\n[fn_spacing]\nmin_statements=5\n",
    )
    .unwrap();
    let (_, original) = capture(directory.path(), &["--guard-clauses", "--parameter-plans"]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    replay.dismiss("many");
    let output = replay.run(&["--guard-clauses", "--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    let report = parsed(&output);
    counts(&report, 1, 0);
    assert_eq!(report["parameter_review"]["hidden_findings"], 0);
    assert_eq!(report["parameter_review"]["visible_findings"], 1);
    assert_eq!(report["analysis"]["findings"], 4);
    let output = replay.run(&[
        "--dismiss-parameter",
        key(&original, "many"),
        "--review-reason",
        "Keep the parameter boundary.",
    ]);
    assert!(output.status.success());
    let output = replay.run(&["--guard-clauses", "--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    let report = parsed(&output);
    counts(&report, 1, 0);
    assert_eq!(report["parameter_review"]["hidden_findings"], 1);
    assert_eq!(report["parameter_review"]["visible_findings"], 0);
    assert_eq!(report["analysis"]["findings"], 4);
    assert_incomplete_guard_planning(&replay);
    let store: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("reviews.json")).unwrap())
            .unwrap();
    assert_eq!(store["schema_version"], 4);
    assert_eq!(store["length_entries"].as_array().unwrap().len(), 1);
    assert_eq!(store["parameter_entries"].as_array().unwrap().len(), 1);
}

fn assert_incomplete_guard_planning(replay: &Replay<'_>) {
    let output = replay.run(&["--guard-clauses", "--parameter-plans", "--fail-on-findings"]);
    let report = parsed(&output);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        report["parameter_planning"]
    );
    assert_eq!(report["parameter_planning"]["findings"], 1);
    assert_eq!(report["parameter_planning"]["coverage"], "incomplete");
    assert!(
        report["parameter_planning"]["caller_census"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|unit| unit["parameter_uses"]["functions"].as_array().unwrap())
            .flat_map(|function| function["boundaries"].as_array().unwrap())
            .any(|boundary| boundary["reason"]
                == "Conditional control flow leaves formal-use coverage incomplete")
    );
}

#[test]
fn invalid_clean_suppressed_unselected_and_unavailable_length_dismissals_never_write() {
    let directory = workspace(&format!(
        "{ORIGINAL}\npub fn clean(value: u32) -> u32 {{ value }}\n// rstyle-allow(fn_length): generated body\npub fn allowed(a: u32) -> u32 {{\n    let next = a + 1;\n    next + 1\n}}\n"
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
        let output = replay.run(&["--dismiss-length", selector, "--review-reason", REASON]);
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
            "--dismiss-length",
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
fn external_module_length_identity_is_explicitly_unavailable() {
    let directory = workspace("pub mod external;\n");
    std::fs::write(directory.path().join("src/external.rs"), ORIGINAL).unwrap();
    let (output, original) = capture(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        original["length_review"]["decisions"][0]["state"],
        "unavailable"
    );
    let output = Replay {
        directory: directory.path(),
        original: &original,
    }
    .run(&[
        "--dismiss-length",
        key(&original, "external::many"),
        "--review-reason",
        REASON,
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!directory.path().join("reviews.json").exists());
}

#[test]
fn adding_length_review_upgrades_a_parameter_store_without_replacing_its_entry() {
    let directory = workspace(ORIGINAL);
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=2\n[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    let (_, original) = capture(directory.path(), &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    let output = replay.run(&[
        "--dismiss-parameter",
        key(&original, "many"),
        "--review-reason",
        "Keep these parameters.",
    ]);
    assert!(output.status.success());
    let file = directory.path().join("reviews.json");
    let parameter_store: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(parameter_store["schema_version"], 3);
    replay.dismiss("many");
    let bytes = std::fs::read(&file).unwrap();
    let combined: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(combined["schema_version"], 4);
    assert_eq!(
        combined["parameter_entries"],
        parameter_store["parameter_entries"]
    );
    let output = replay.run(&["--fail-on-findings", "--parameter-plans"]);
    assert!(output.status.success());
    let report = parsed(&output);
    counts(&report, 1, 0);
    assert_eq!(report["parameter_review"]["hidden_findings"], 1);
    assert_eq!(report["analysis"]["visible_findings"], 2);
    assert_eq!(report["parameter_planning"]["findings"], 1);
    let output = replay.run(&[
        "--dismiss-length",
        key(&original, "many"),
        "--review-reason",
        REASON,
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}
