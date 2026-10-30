use super::*;

pub(super) const COMPARE: &[&str] = &[
    "--compiler-advisory-config",
    "policy.toml",
    "--compiler-baseline",
    "baseline.json",
    "--fail-on-new-compiler",
];

fn baseline(report: &Value) -> &Value {
    &report["compiler_advisory"]["diagnostic_baseline"]
}

pub(super) fn exported() -> (TempDir, Value) {
    let (directory, original) = configured();
    let (output, report) = replay(
        directory.path(),
        &original,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--write-compiler-baseline",
            "baseline.json",
        ],
    );
    checked(&output, 0);
    (directory, report)
}

fn checked(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}\n{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn raw_replay(directory: &Path, original: &Value, extra: &[&str]) -> Output {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .args(extra)
        .env("PATH", directory.join("no-tools"));
    diagnostic_process::output(&mut command).unwrap()
}

#[test]
fn six_family_baseline_replays_without_sources_and_preserves_raw_findings() {
    let (directory, original) = exported();
    for path in ["src/lib.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    let bytes = std::fs::read(directory.path().join("baseline.json")).unwrap();
    let saved: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(saved["schema"], "rstyle.compiler-baseline/1");
    assert_eq!(saved["entries"].as_array().unwrap().len(), 6);
    assert!(!directory.path().join("reviews.json").exists());
    let wire = String::from_utf8(bytes).unwrap();
    for absent in ["sha256", "fingerprint", "diagnostic_index", "graph", "body"] {
        assert!(!wire.contains(absent), "{absent}");
    }
    for detail in ["full", "compact"] {
        let mut args = COMPARE.to_vec();
        args.extend(["--report-detail", detail]);
        let output = raw_replay(directory.path(), &original, &args);
        checked(&output, 0);
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(baseline(&report)["existing_findings"], 6);
        assert_eq!(baseline(&report)["new_findings"], 0);
        assert_eq!(baseline(&report)["coverage"], "complete");
        assert_eq!(report["matching"], original["matching"]);
        assert_eq!(
            report["compiler_advisory"]["units"],
            original["compiler_advisory"]["units"]
        );
        args.push("--fail-on-findings");
        checked(&raw_replay(directory.path(), &original, &args), 1);
    }
}

#[test]
fn historical_baseline_metrics_remain_readable_and_mark_scope_drift() {
    let (directory, original) = exported();
    let path = directory.path().join("baseline.json");
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for version in [
        "rstyle.compiler-style-metrics/1",
        "rstyle.compiler-style-metrics/2",
        "rstyle.compiler-style-metrics/3",
        "rstyle.compiler-style-metrics/4",
    ] {
        saved["metrics"] = version.into();
        for entry in saved["entries"].as_array_mut().unwrap() {
            entry["metrics"] = version.into();
        }
        std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
        let (output, report) = replay(directory.path(), &original, COMPARE);
        checked(&output, 1);
        assert_eq!(baseline(&report)["scope_changed"], true);
        assert_eq!(baseline(&report)["existing_findings"], 0);
        assert_eq!(baseline(&report)["new_findings"], 6);
        assert_eq!(baseline(&report)["coverage"], "complete");
        assert!(
            baseline(&report)["retained"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["state"] == "changed")
        );
    }
}

#[test]
fn compiler_baseline_movement_preserves_equality_and_new_measurements_fail() {
    let (directory, _) = exported();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("\n// Move owner\n{SOURCE}"),
    )
    .unwrap();
    let (output, moved) = capture_in(directory.path(), COMPARE);
    checked(&output, 0);
    assert_eq!(baseline(&moved)["existing_findings"], 6);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        SOURCE.replace("x+=2;", "x+=2;\nx+=4;"),
    )
    .unwrap();
    let (output, changed) = capture_in(directory.path(), COMPARE);
    checked(&output, 1);
    assert!(baseline(&changed)["new_findings"].as_u64().unwrap() > 0);
    assert!(
        baseline(&changed)["current"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["state"] == "changed")
    );
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{SOURCE}\n{}", SOURCE.replace("mixed(", "additional(")),
    )
    .unwrap();
    let (output, added) = capture_in(directory.path(), COMPARE);
    checked(&output, 1);
    assert_eq!(baseline(&added)["existing_findings"], 6);
    assert_eq!(baseline(&added)["new_findings"], 6);
}

pub(super) fn capture_in(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec!["--extraction-profile", "hir-data-v3", "--compiler-advisory"];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

#[test]
fn empty_baselines_still_detect_policy_and_compilation_context_drift() {
    let (directory, _, original) = capture(
        "pub fn empty(){}",
        &["--write-compiler-baseline", "baseline.json"],
    );
    let (output, same) = replay(
        directory.path(),
        &original,
        &[
            "--compiler-baseline",
            "baseline.json",
            "--fail-on-new-compiler",
        ],
    );
    checked(&output, 0);
    assert_eq!(baseline(&same)["new_findings"], 0);
    std::fs::write(directory.path().join("policy.toml"), CONFIG).unwrap();
    let (output, drift) = replay(directory.path(), &original, COMPARE);
    checked(&output, 1);
    assert_eq!(baseline(&drift)["scope_changed"], true);
    let (output, contexts) = capture_in(
        directory.path(),
        &[
            "--all-targets",
            "--compiler-baseline",
            "baseline.json",
            "--fail-on-new-compiler",
        ],
    );
    checked(&output, 2);
    assert_eq!(baseline(&contexts)["coverage"], "incomplete");
    assert_eq!(baseline(&contexts)["scope_changed"], true);
    assert_eq!(baseline(&contexts)["new_findings"], 0);
}

#[test]
fn empty_baseline_metric_drift_cannot_establish_a_clean_gate() {
    let (directory, _, original) = capture(
        "pub fn empty(){}",
        &["--write-compiler-baseline", "baseline.json"],
    );
    let path = directory.path().join("baseline.json");
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["entries"], serde_json::json!([]));
    saved["metrics"] = "rstyle.compiler-style-metrics/4".into();
    std::fs::write(path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let (output, report) = replay(
        directory.path(),
        &original,
        &[
            "--compiler-baseline",
            "baseline.json",
            "--fail-on-new-compiler",
        ],
    );
    checked(&output, 1);
    assert_eq!(baseline(&report)["scope_changed"], true);
    assert_eq!(baseline(&report)["new_findings"], 0);
}

#[test]
fn release_context_drift_fails_even_with_no_findings() {
    let (directory, _, _) = capture(
        "pub fn empty(){}",
        &["--write-compiler-baseline", "baseline.json"],
    );
    let (output, report) = capture_in(
        directory.path(),
        &[
            "--release",
            "--compiler-baseline",
            "baseline.json",
            "--fail-on-new-compiler",
        ],
    );
    checked(&output, 1);
    assert_eq!(baseline(&report)["coverage"], "complete");
    assert_eq!(baseline(&report)["scope_changed"], true);
    assert_eq!(baseline(&report)["new_findings"], 0);
}

#[test]
fn invalid_suppression_diagnostics_cannot_be_exported_as_a_clean_baseline() {
    let (directory, _, original) = capture(
        "// rstyle-allow(unknown): invalid fixture\npub fn original(){}",
        &[],
    );
    let output = raw_replay(
        directory.path(),
        &original,
        &["--write-compiler-baseline", "baseline.json"],
    );
    checked(&output, 2);
    assert!(!directory.path().join("baseline.json").exists());
}

#[test]
fn completed_same_policy_negatives_resolve_prior_compiler_findings() {
    let (directory, _) = exported();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn mixed()->u128 {0}",
    )
    .unwrap();
    let (output, gone) = capture_in(directory.path(), COMPARE);
    checked(&output, 0);
    let retained = baseline(&gone)["retained"].as_array().unwrap();
    assert_eq!(retained.len(), 6);
    assert!(retained.iter().all(|row| row["state"] == "resolved"));
}

#[test]
fn reviewed_new_findings_do_not_fail_baseline_gate_or_disappear_from_export() {
    let (directory, original) = configured();
    for index in 0..6 {
        dismiss(directory.path(), &original, index);
    }
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn empty(){}").unwrap();
    let (output, _) = capture_in(
        directory.path(),
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--write-compiler-baseline",
            "empty.json",
        ],
    );
    checked(&output, 0);
    let (output, report) = replay(
        directory.path(),
        &original,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--compiler-baseline",
            "empty.json",
            "--fail-on-new-compiler",
            "--review-file",
            "reviews.json",
        ],
    );
    checked(&output, 0);
    assert_eq!(baseline(&report)["new_findings"], 0);
    assert_eq!(baseline(&report)["existing_findings"], 0);
    assert_eq!(baseline(&report)["current"].as_array().unwrap().len(), 6);
    assert!(
        baseline(&report)["current"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["review_hidden"] == true)
    );
    assert_reviewed_export(directory.path(), &original);
}

#[test]
fn incomplete_inputs_cannot_publish_or_become_clean_via_baseline() {
    let source = format!("{SOURCE}\nmacro_rules! make {{()=>{{pub fn generated(){{}}}}}} make!();");
    let (directory, _) = exported();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, original) = capture_in(directory.path(), COMPARE);
    checked(&output, 2);
    assert_eq!(baseline(&original)["coverage"], "incomplete");
    assert_eq!(baseline(&original)["existing_findings"], 6);
    let output = raw_replay(
        directory.path(),
        &original,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--write-compiler-baseline",
            "incomplete.json",
        ],
    );
    checked(&output, 2);
    assert!(!directory.path().join("incomplete.json").exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Incomplete analysis"));
}

#[test]
fn invalid_baseline_headers_rosters_and_existing_destinations_are_rejected() {
    let (directory, original) = exported();
    let path = directory.path().join("baseline.json");
    let bytes = std::fs::read(&path).unwrap();
    let saved: Value = serde_json::from_slice(&bytes).unwrap();
    let mut variants = vec![saved.clone(); 5];
    variants[0]["metrics"] = "future/9".into();
    variants[1]["contexts"] = serde_json::json!([]);
    variants[2]["entries"]
        .as_array_mut()
        .unwrap()
        .push(saved["entries"][0].clone());
    variants[3]["entries"][0]["actual"] = 0.into();
    variants[4]["unexpected"] = true.into();
    for variant in variants {
        std::fs::write(&path, serde_json::to_vec(&variant).unwrap()).unwrap();
        checked(&raw_replay(directory.path(), &original, COMPARE), 2);
    }
    std::fs::write(&path, &bytes).unwrap();
    checked(
        &raw_replay(
            directory.path(),
            &original,
            &[
                "--compiler-advisory-config",
                "policy.toml",
                "--write-compiler-baseline",
                "baseline.json",
            ],
        ),
        2,
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

fn assert_reviewed_export(directory: &Path, original: &Value) {
    let output = raw_replay(
        directory,
        original,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--review-file",
            "reviews.json",
            "--write-compiler-baseline",
            "reviewed.json",
        ],
    );
    checked(&output, 0);
    let saved: Value =
        serde_json::from_slice(&std::fs::read(directory.join("reviewed.json")).unwrap()).unwrap();
    assert_eq!(saved["entries"].as_array().unwrap().len(), 6);
}

#[test]
fn compilation_units_without_functions_do_not_invent_empty_build_contexts() {
    let (directory, output, original) = capture("", &[]);
    checked(&output, 0);
    let output = raw_replay(
        directory.path(),
        &original,
        &["--write-compiler-baseline", "baseline.json"],
    );
    checked(&output, 2);
    assert!(!directory.path().join("baseline.json").exists());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Baseline build context is unavailable")
    );
}
