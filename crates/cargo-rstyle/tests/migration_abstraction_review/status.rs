use super::*;

fn status_records(report: &Value) -> &[Value] {
    assert_eq!(report["review_status"]["schema"], "rstyle.review-status/1");
    report["review_status"]["records"].as_array().unwrap()
}

fn assert_record(record: &Value, family: &str, state: &str, index: usize) {
    assert_eq!(record["family"], family);
    assert_eq!(record["state"], state);
    assert_eq!(record["entry_index"], index);
    assert!(!record["reason"].as_str().unwrap().is_empty());
    assert!(!record["detail"].as_str().unwrap().is_empty());
    assert!(!record["members"].as_array().unwrap().is_empty());
}

#[test]
fn retained_status_is_source_free_exact_and_identical_in_full_and_compact_reports() {
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
        let mut statuses = Vec::new();
        for detail in ["full", "compact"] {
            let output = replay.run(&[
                "--review-status",
                "--fail-on-findings",
                "--report-detail",
                detail,
            ]);
            assert!(output.status.success(), "{}", stdout(&output));
            let report = parsed(&output);
            counts(&report, 1, 0);
            let records = status_records(&report);
            assert_eq!(records.len(), 1);
            assert_record(
                &records[0],
                case(id)["family"].as_str().unwrap(),
                "hidden_exact",
                0,
            );
            statuses.push(report["review_status"].clone());
        }
        assert_eq!(statuses[0], statuses[1]);
        assert_eq!(std::fs::read(file).unwrap(), bytes);
    }
}

#[test]
fn completed_negative_dispatch_is_assessed_as_no_finding() {
    let directory = workspace("dispatch");
    let original = capture(directory.path(), "dispatch", &[]);
    let file = directory.path().join("reviews.json");
    Replay {
        directory: directory.path(),
        original: &original,
        file: &file,
        id: "dispatch",
    }
    .dismiss();
    let source = case("dispatch")["source"].as_str().unwrap().replace(
        "match item { Item::A(a)=>a.measure(), Item::B(b)=>b.amount() }",
        "0",
    );
    write_source(directory.path(), &source);
    let changed = capture(directory.path(), "dispatch", &[]);
    assert_negative_dispatch(&changed);
    let bytes = std::fs::read(&file).unwrap();
    let output = Replay {
        directory: directory.path(),
        original: &changed,
        file: &file,
        id: "dispatch",
    }
    .run(&["--review-status", "--fail-on-findings"]);
    assert!(output.status.success(), "{}", stdout(&output));
    let report = parsed(&output);
    assert_record(
        &status_records(&report)[0],
        "enum_dispatch",
        "no_finding",
        0,
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn missing_endpoint_is_unobserved_and_only_requested_status_makes_it_incomplete() {
    let directory = workspace("dispatch");
    let original = capture(directory.path(), "dispatch", &[]);
    let file = directory.path().join("reviews.json");
    Replay {
        directory: directory.path(),
        original: &original,
        file: &file,
        id: "dispatch",
    }
    .dismiss();
    let source = case("dispatch")["source"]
        .as_str()
        .unwrap()
        .replace("pub fn dispatch(", "pub fn relocated(");
    write_source(directory.path(), &source);
    let changed = capture(directory.path(), "dispatch", &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &changed,
        file: &file,
        id: "dispatch",
    };
    let ordinary = replay.run(&[]);
    assert!(ordinary.status.success(), "{}", stdout(&ordinary));
    assert!(parsed(&ordinary).get("review_status").is_none());
    let bytes = std::fs::read(&file).unwrap();
    let output = replay.run(&["--review-status"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_record(
        &status_records(&parsed(&output))[0],
        "enum_dispatch",
        "not_observed",
        0,
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn mixed_store_status_marks_omitted_family_without_cross_family_hiding() {
    let directory = workspace("capability");
    write_source(
        directory.path(),
        &format!(
            "pub mod capabilities {{ {} }} pub mod dispatches {{ {} }}",
            case("capability")["source"].as_str().unwrap(),
            case("dispatch")["source"].as_str().unwrap()
        ),
    );
    let original = capture(directory.path(), "capability", &["--enum-dispatch"]);
    let file = directory.path().join("reviews.json");
    for id in ["capability", "dispatch"] {
        Replay {
            directory: directory.path(),
            original: &original,
            file: &file,
            id,
        }
        .dismiss();
    }
    let bytes = std::fs::read(&file).unwrap();
    let output = Replay {
        directory: directory.path(),
        original: &original,
        file: &file,
        id: "capability",
    }
    .run(&["--review-status", "--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let report = parsed(&output);
    let records = status_records(&report);
    assert_eq!(records.len(), 2);
    assert_record(&records[0], "capability_set", "hidden_exact", 0);
    assert_record(&records[1], "enum_dispatch", "not_requested", 1);
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn changed_roles_are_stale_and_budget_exhaustion_never_claims_a_negative() {
    for id in ["dispatch", "capability"] {
        let directory = workspace(id);
        let original = capture(directory.path(), id, &[]);
        let file = directory.path().join("reviews.json");
        Replay {
            directory: directory.path(),
            original: &original,
            file: &file,
            id,
        }
        .dismiss();
        write_source(directory.path(), &role_edit(id));
        let changed = capture(directory.path(), id, &[]);
        let replay = Replay {
            directory: directory.path(),
            original: &changed,
            file: &file,
            id,
        };
        let bytes = std::fs::read(&file).unwrap();
        let output = replay.run(&["--review-status", "--fail-on-findings"]);
        assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
        let report = parsed(&output);
        assert_record(
            &status_records(&report)[0],
            case(id)["family"].as_str().unwrap(),
            "stale",
            0,
        );
        let output = replay.run(&["--review-status", "--max-analysis-functions", "0"]);
        assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
        let report = parsed(&output);
        let state = status_records(&report)[0]["state"].as_str().unwrap();
        assert!(matches!(state, "unavailable" | "not_observed"), "{state}");
        assert_eq!(std::fs::read(file).unwrap(), bytes);
    }
}

#[test]
fn helper_schema_one_exact_status_does_not_migrate_or_rewrite() {
    let directory = workspace("capability");
    write_source(
        directory.path(),
        "pub fn first(x:u32)->u32 {(x+1)*2} pub fn second(y:u32)->u32 {(y+3)*2}",
    );
    let (output, original) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--helper-proposals"],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let file = directory.path().join("reviews.json");
    let output = helper_replay(
        directory.path(),
        &original,
        &file,
        &[
            "--dismiss-helper-pair",
            key(&original, "first"),
            key(&original, "second"),
            "--review-reason",
            "Keep helper behavior.",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let bytes = std::fs::read(&file).unwrap();
    let store: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(store["schema_version"], 1);
    let output = helper_replay(
        directory.path(),
        &original,
        &file,
        &["--review-status", "--fail-on-findings"],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let report = parsed(&output);
    assert_record(&status_records(&report)[0], "helper", "hidden_exact", 0);
    assert_eq!(report["review"]["pairs"][0]["state"], "hidden_exact");
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn changed_context_is_assessed_even_when_the_current_caller_has_no_finding() {
    let directory = workspace("dispatch");
    let manifest = directory.path().join("Cargo.toml");
    let manifest_text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, manifest_text + "\n[features]\nextra=[]\n").unwrap();
    let original = capture(directory.path(), "dispatch", &[]);
    let file = directory.path().join("reviews.json");
    Replay {
        directory: directory.path(),
        original: &original,
        file: &file,
        id: "dispatch",
    }
    .dismiss();
    let source = case("dispatch")["source"].as_str().unwrap().replace(
        "match item { Item::A(a)=>a.measure(), Item::B(b)=>b.amount() }",
        "0",
    );
    write_source(directory.path(), &source);
    let changed = capture(directory.path(), "dispatch", &["--features", "extra"]);
    assert_negative_dispatch(&changed);
    let bytes = std::fs::read(&file).unwrap();
    let output = Replay {
        directory: directory.path(),
        original: &changed,
        file: &file,
        id: "dispatch",
    }
    .run(&["--review-status", "--fail-on-findings"]);
    assert!(output.status.success(), "{}", stdout(&output));
    let report = parsed(&output);
    assert_record(
        &status_records(&report)[0],
        "enum_dispatch",
        "context_changed",
        0,
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

fn assert_negative_dispatch(report: &Value) {
    assert_eq!(report["enum_dispatch"]["findings"], 0);
    let caller = report["enum_dispatch"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"].as_str().unwrap().ends_with("dispatch"))
        .unwrap();
    assert_eq!(caller["availability"], "known");
    assert_eq!(caller["outcome"], "evaluated_no_finding");
    assert_eq!(caller["execution"], "completed");
    assert_eq!(caller["coverage"], "complete");
}

#[test]
fn unrecognized_retained_identity_version_cannot_claim_a_completed_negative() {
    let directory = workspace("dispatch");
    let original = capture(directory.path(), "dispatch", &[]);
    let file = directory.path().join("reviews.json");
    Replay {
        directory: directory.path(),
        original: &original,
        file: &file,
        id: "dispatch",
    }
    .dismiss();
    let mut store: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    store["source_entries"][0]["finding"]["versions"]["identity_version"] = 2.into();
    let bytes = serde_json::to_vec(&store).unwrap();
    std::fs::write(&file, &bytes).unwrap();
    let source = case("dispatch")["source"].as_str().unwrap().replace(
        "match item { Item::A(a)=>a.measure(), Item::B(b)=>b.amount() }",
        "0",
    );
    write_source(directory.path(), &source);
    let changed = capture(directory.path(), "dispatch", &[]);
    assert_negative_dispatch(&changed);
    let output = Replay {
        directory: directory.path(),
        original: &changed,
        file: &file,
        id: "dispatch",
    }
    .run(&["--review-status"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_record(
        &status_records(&parsed(&output))[0],
        "enum_dispatch",
        "unavailable",
        0,
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn excluded_helper_endpoints_remain_unavailable_instead_of_becoming_negative() {
    let directory = workspace("capability");
    write_source(
        directory.path(),
        "pub fn first(x:u32)->u32 {(x+1)*2} pub fn second(y:u32)->u32 {(y+3)*2}",
    );
    let (output, original) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--helper-proposals"],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let file = directory.path().join("reviews.json");
    dismiss_helper(directory.path(), &original, &file);
    let bytes = std::fs::read(&file).unwrap();
    write_source(
        directory.path(),
        "pub fn first(x:u32)->u32 {let read=||x;read()} pub fn second(y:u32)->u32 {let read=||y;read()}",
    );
    let (output, changed) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--helper-proposals"],
    );
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(changed["helper_proposals"]["findings"], 0);
    assert!(
        !changed["helper_proposals"]["unavailable_functions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let output = helper_replay(directory.path(), &changed, &file, &["--review-status"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_record(
        &status_records(&parsed(&output))[0],
        "helper",
        "unavailable",
        0,
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn exact_existing_trait_suppression_assesses_a_retained_capability_as_no_finding() {
    let directory = workspace("capability");
    let original = capture(directory.path(), "capability", &[]);
    let file = directory.path().join("reviews.json");
    Replay {
        directory: directory.path(),
        original: &original,
        file: &file,
        id: "capability",
    }
    .dismiss();
    let bytes = std::fs::read(&file).unwrap();
    let labels: Value = serde_json::from_str(include_str!(
        "../../../../eval/clones-capability/capability-cases.json"
    ))
    .unwrap();
    let source = labels["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "exact-existing-trait")
        .unwrap()["source"]
        .as_str()
        .unwrap();
    write_source(directory.path(), source);
    let changed = capture(directory.path(), "capability", &[]);
    assert_eq!(changed["capability_proposals"]["findings"], 0);
    assert_eq!(
        changed["capability_proposals"]["suppressed"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let output = Replay {
        directory: directory.path(),
        original: &changed,
        file: &file,
        id: "capability",
    }
    .run(&["--review-status", "--fail-on-findings"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_record(
        &status_records(&parsed(&output))[0],
        "capability_set",
        "no_finding",
        0,
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn status_cli_requires_a_review_file_and_rejects_dismissal_without_writing() {
    let directory = workspace("dispatch");
    let file = directory.path().join("reviews.json");
    let executable = cargo_bin("cargo-rstyle").canonicalize().unwrap();
    for arguments in [
        vec!["--review-status"],
        vec![
            "--review-status",
            "--review-file",
            file.to_str().unwrap(),
            "--dismiss-enum-dispatch",
            "function:u0:f0",
        ],
    ] {
        let output = diagnostic_process::output(
            Command::new(&executable)
                .current_dir(directory.path())
                .args(["clones"])
                .args(arguments)
                .env("PATH", directory.path().join("no-tools")),
        )
        .unwrap();
        assert_eq!(output.status.code(), Some(2));
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("--review-status") || error.contains("--review-file"),
            "{error}"
        );
        assert!(!file.exists());
    }
}

fn dismiss_helper(directory: &Path, original: &Value, file: &Path) {
    let output = helper_replay(
        directory,
        original,
        file,
        &[
            "--dismiss-helper-pair",
            key(original, "first"),
            key(original, "second"),
            "--review-reason",
            "Keep helper behavior.",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
}
