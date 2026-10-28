use super::*;

fn negative_dispatch_source() -> String {
    case("dispatch")["source"].as_str().unwrap().replace(
        "match item { Item::A(a)=>a.measure(), Item::B(b)=>b.amount() }",
        "0",
    )
}

fn prune_report(report: &Value, dry_run: bool, eligible: usize, retained: usize) {
    let removed = if dry_run { 0 } else { eligible };
    let prune = &report["review_prune"];
    assert_eq!(prune["schema"], "rstyle.review-prune/1");
    assert_eq!(prune["dry_run"], dry_run);
    assert_eq!(prune["eligible_entries"], eligible);
    assert_eq!(prune["removed_entries"], removed);
    assert_eq!(prune["retained_entries"], retained);
    assert_eq!(prune["selections"].as_array().unwrap().len(), eligible);
    assert_eq!(report["review_status"]["schema"], "rstyle.review-status/1");
}

fn dismiss_dispatch(directory: &Path, file: &Path) -> Value {
    let original = capture(directory, "dispatch", &[]);
    Replay {
        directory,
        original: &original,
        file,
        id: "dispatch",
    }
    .dismiss();
    original
}

#[test]
fn completed_negative_dispatch_pruning_previews_applies_and_is_idempotent() {
    let directory = workspace("dispatch");
    let file = directory.path().join("reviews.json");
    dismiss_dispatch(directory.path(), &file);
    let bytes = std::fs::read(&file).unwrap();
    write_source(directory.path(), &negative_dispatch_source());
    let changed = capture(directory.path(), "dispatch", &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &changed,
        file: &file,
        id: "dispatch",
    };
    let preview = replay.run(&["--review-prune", "--review-prune-dry-run"]);
    assert!(preview.status.success(), "{}", stdout(&preview));
    let report = parsed(&preview);
    prune_report(&report, true, 1, 1);
    assert_eq!(report["review_status"]["records"][0]["state"], "no_finding");
    assert_eq!(
        report["review_prune"]["selections"][0],
        serde_json::json!({"family":"enum_dispatch","entry_index":0})
    );
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    let output = replay.run(&["--review-prune"]);
    assert!(output.status.success(), "{}", stdout(&output));
    let report = parsed(&output);
    prune_report(&report, false, 1, 0);
    assert_eq!(report["review_status"]["records"][0]["state"], "no_finding");
    let applied = std::fs::read(&file).unwrap();
    let store: Value = serde_json::from_slice(&applied).unwrap();
    assert_eq!(store["schema_version"], 2);
    assert!(store["entries"].as_array().unwrap().is_empty());
    assert!(
        store
            .get("source_entries")
            .is_none_or(|entries| entries.as_array().unwrap().is_empty())
    );
    let output = replay.run(&["--review-prune"]);
    assert!(output.status.success(), "{}", stdout(&output));
    prune_report(&parsed(&output), false, 0, 0);
    assert_eq!(std::fs::read(file).unwrap(), applied);
}

#[test]
fn source_free_current_findings_are_retained_in_identical_full_and_compact_previews() {
    for id in ["dispatch", "capability"] {
        let directory = workspace(id);
        let original = capture(directory.path(), id, &[]);
        let file = directory.path().join("reviews.json");
        let replay = Replay {
            directory: directory.path(),
            original: &original,
            file: &file,
            id,
        };
        replay.dismiss();
        let bytes = std::fs::read(&file).unwrap();
        std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
        std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
        let mut reports = Vec::new();
        for detail in ["full", "compact"] {
            let output = replay.run(&[
                "--review-prune",
                "--review-prune-dry-run",
                "--report-detail",
                detail,
                "--fail-on-findings",
            ]);
            assert!(output.status.success(), "{}", stdout(&output));
            let report = parsed(&output);
            prune_report(&report, true, 0, 1);
            assert_eq!(
                report["review_status"]["records"][0]["state"],
                "hidden_exact"
            );
            reports.push((
                report["review_prune"].clone(),
                report["review_status"].clone(),
            ));
        }
        assert_eq!(reports[0], reports[1]);
        let output = replay.run(&["--review-prune"]);
        assert!(output.status.success(), "{}", stdout(&output));
        prune_report(&parsed(&output), false, 0, 1);
        assert_eq!(std::fs::read(file).unwrap(), bytes);
    }
}

#[test]
fn missing_review_store_is_not_created_by_preview_or_empty_apply() {
    let directory = workspace("dispatch");
    let original = capture(directory.path(), "dispatch", &[]);
    let file = directory.path().join("absent.json");
    let replay = Replay {
        directory: directory.path(),
        original: &original,
        file: &file,
        id: "dispatch",
    };
    for extra in [
        vec!["--review-prune", "--review-prune-dry-run"],
        vec!["--review-prune"],
    ] {
        let output = replay.run(&extra);
        assert!(output.status.success(), "{}", stdout(&output));
        prune_report(&parsed(&output), extra.len() == 2, 0, 0);
        assert!(!file.exists());
    }
}

fn dismiss_both_families(directory: &Path, file: &Path, original: &Value) {
    for id in ["capability", "dispatch"] {
        Replay {
            directory,
            original,
            file,
            id,
        }
        .dismiss();
    }
}

#[test]
fn selective_pruning_preserves_omitted_family_and_incomplete_exit_status() {
    let directory = workspace("capability");
    let source = format!(
        "pub mod capabilities {{ {} }} pub mod dispatches {{ {} }}",
        case("capability")["source"].as_str().unwrap(),
        case("dispatch")["source"].as_str().unwrap()
    );
    write_source(directory.path(), &source);
    let original = capture(directory.path(), "capability", &["--enum-dispatch"]);
    let file = directory.path().join("reviews.json");
    dismiss_both_families(directory.path(), &file, &original);
    let before: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    write_source(
        directory.path(),
        &source.replace(
            "match item { Item::A(a)=>a.measure(), Item::B(b)=>b.amount() }",
            "0",
        ),
    );
    let changed = capture(directory.path(), "dispatch", &[]);
    let output = Replay {
        directory: directory.path(),
        original: &changed,
        file: &file,
        id: "dispatch",
    }
    .run(&["--review-prune"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let report = parsed(&output);
    prune_report(&report, false, 1, 1);
    assert_eq!(
        report["review_status"]["records"][0]["state"],
        "not_requested"
    );
    assert_eq!(report["review_status"]["records"][1]["state"], "no_finding");
    assert_eq!(
        report["review_prune"]["selections"][0],
        serde_json::json!({"family":"enum_dispatch","entry_index":1})
    );
    let after: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(after["schema_version"], before["schema_version"]);
    assert_eq!(after["entries"], before["entries"]);
    assert_eq!(
        after["source_entries"],
        serde_json::json!([before["source_entries"][0].clone()])
    );
}

fn assert_missing_dispatch_retained(directory: &Path, file: &Path, bytes: &[u8]) {
    let missing = case("dispatch")["source"]
        .as_str()
        .unwrap()
        .replace("pub fn dispatch(", "pub fn relocated(");
    write_source(directory, &missing);
    let changed = capture(directory, "dispatch", &[]);
    let output = Replay {
        directory,
        original: &changed,
        file,
        id: "dispatch",
    }
    .run(&["--review-prune"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let report = parsed(&output);
    prune_report(&report, false, 0, 1);
    assert_eq!(
        report["review_status"]["records"][0]["state"],
        "not_observed"
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn unknown_versions_missing_endpoints_and_exhausted_budgets_never_prune() {
    let directory = workspace("dispatch");
    let file = directory.path().join("reviews.json");
    let original = dismiss_dispatch(directory.path(), &file);
    let bytes = std::fs::read(&file).unwrap();
    let replay = Replay {
        directory: directory.path(),
        original: &original,
        file: &file,
        id: "dispatch",
    };
    let output = replay.run(&["--review-prune", "--max-analysis-functions", "0"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    prune_report(&parsed(&output), false, 0, 1);
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    assert_missing_dispatch_retained(directory.path(), &file, &bytes);
    let mut store: Value = serde_json::from_slice(&bytes).unwrap();
    store["source_entries"][0]["finding"]["versions"]["identity_version"] = 2.into();
    let unknown = serde_json::to_vec(&store).unwrap();
    std::fs::write(&file, &unknown).unwrap();
    write_source(directory.path(), &negative_dispatch_source());
    let changed = capture(directory.path(), "dispatch", &[]);
    let output = Replay {
        directory: directory.path(),
        original: &changed,
        file: &file,
        id: "dispatch",
    }
    .run(&["--review-prune"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let report = parsed(&output);
    prune_report(&report, false, 0, 1);
    assert_eq!(
        report["review_status"]["records"][0]["state"],
        "unavailable"
    );
    assert_eq!(std::fs::read(file).unwrap(), unknown);
}

fn assert_prune_rejected(directory: &Path, arguments: &[&str]) {
    let executable = cargo_bin("cargo-rstyle").canonicalize().unwrap();
    let output = diagnostic_process::output(
        Command::new(executable)
            .current_dir(directory)
            .arg("clones")
            .args(arguments)
            .env("PATH", directory.join("no-tools")),
    )
    .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("--review-prune")
            || error.contains("--review-file")
            || error.contains("--replay-run"),
        "{error}"
    );
}

#[test]
fn prune_cli_requires_saved_replay_and_store_and_rejects_dismissals() {
    let directory = workspace("dispatch");
    let file = directory.path().join("reviews.json");
    let path = file.to_str().unwrap();
    for arguments in [
        vec!["--review-prune"],
        vec!["--review-prune", "--review-file", path],
        vec!["--review-prune-dry-run", "--review-file", path],
    ] {
        assert_prune_rejected(directory.path(), &arguments);
    }
    for selector in [
        vec!["--dismiss-enum-dispatch", "function:u0:f0"],
        vec![
            "--dismiss-capability-pair",
            "function:u0:f0",
            "function:u0:f1",
        ],
        vec!["--dismiss-helper-pair", "function:u0:f0", "function:u0:f1"],
    ] {
        let mut arguments = vec![
            "--review-prune",
            "--review-file",
            path,
            "--replay-run",
            "absent",
            "--review-reason",
            "Keep it",
        ];
        arguments.extend(selector);
        assert_prune_rejected(directory.path(), &arguments);
    }
    assert!(!file.exists());
}

fn dismiss_helper(directory: &Path, file: &Path) {
    write_source(
        directory,
        "pub fn first(x:u32)->u32 {(x+1)*2} pub fn second(y:u32)->u32 {(y+3)*2}",
    );
    let (output, original) = clones(
        directory,
        &["--extraction-profile", "hir-data-v3", "--helper-proposals"],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let output = helper_replay(
        directory,
        &original,
        file,
        &[
            "--dismiss-helper-pair",
            key(&original, "first"),
            key(&original, "second"),
            "--review-reason",
            "Retain this helper separately.",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
}

#[test]
fn helper_schema_one_completed_negative_is_pruned_without_schema_migration() {
    let directory = workspace("capability");
    let file = directory.path().join("reviews.json");
    dismiss_helper(directory.path(), &file);
    let bytes = std::fs::read(&file).unwrap();
    write_source(
        directory.path(),
        "pub fn first(x:u32)->u32 {x+1} pub fn second(y:u32)->u32 {y+3}",
    );
    let (output, changed) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--helper-proposals"],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(changed["helper_proposals"]["findings"], 0);
    let output = helper_replay(
        directory.path(),
        &changed,
        &file,
        &["--review-prune", "--review-prune-dry-run"],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let report = parsed(&output);
    prune_report(&report, true, 1, 1);
    assert_eq!(report["review_status"]["records"][0]["state"], "no_finding");
    assert_eq!(
        report["review_prune"]["selections"][0],
        serde_json::json!({"family":"helper","entry_index":0})
    );
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    let output = helper_replay(directory.path(), &changed, &file, &["--review-prune"]);
    assert!(output.status.success(), "{}", stdout(&output));
    prune_report(&parsed(&output), false, 1, 0);
    let store: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(store["schema_version"], 1);
    assert!(store["entries"].as_array().unwrap().is_empty());
    assert!(store.get("source_entries").is_none());
}

#[test]
fn failed_saved_capture_validation_blocks_prune_publication() {
    let directory = workspace("dispatch");
    let file = directory.path().join("reviews.json");
    dismiss_dispatch(directory.path(), &file);
    write_source(directory.path(), &negative_dispatch_source());
    let changed = capture(directory.path(), "dispatch", &[]);
    let bytes = std::fs::read(&file).unwrap();
    let missing_run = directory.path().join("missing-run");
    let mut invalid = changed.clone();
    invalid["run_directory"] = missing_run.to_str().unwrap().into();
    let output = Replay {
        directory: directory.path(),
        original: &invalid,
        file: &file,
        id: "dispatch",
    }
    .run(&["--review-prune"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}
