use super::*;

const ORIGINAL: &str = "pub fn update(target:&mut u32){*target=1;}\n";
const BASELINE: &[&str] = &[
    "--density-baseline",
    "baseline.json",
    "--fail-on-new-density",
];

fn parsed(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)))
}

fn checked(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "stdout: {}\nstderr: {}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn counts(report: &Value, existing: usize, new: usize) {
    let baseline = &report["density_review"]["baseline"];
    assert_eq!(baseline["coverage"], "complete", "{baseline}");
    assert_eq!(baseline["existing_findings"], existing, "{baseline}");
    assert_eq!(baseline["new_findings"], new, "{baseline}");
}

fn exported(source: &str) -> (TempDir, Value) {
    let directory = workspace(source, POLICY);
    let (output, report) = requested(
        directory.path(),
        &["--write-density-baseline", "baseline.json"],
    );
    checked(&output, 0);
    assert!(directory.path().join("baseline.json").is_file());
    assert!(!directory.path().join("reviews.json").exists());
    (directory, report)
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> Output {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command.current_dir(directory).args([
        "clones",
        "--format",
        "json",
        "--extraction-profile",
        "hir-data-v3",
        "--effect-density",
        "--effect-config",
        "effects.toml",
        "--replay-run",
        original["run_directory"].as_str().unwrap(),
    ]);
    command
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    diagnostic_process::output(&mut command).unwrap()
}

#[test]
fn exact_density_baseline_is_not_a_saved_review_and_preserves_the_existing_findings_gate() {
    let (directory, original) = exported(ORIGINAL);
    let output = replay(directory.path(), &original, BASELINE);
    checked(&output, 0);
    let report = parsed(&output);
    counts(&report, 1, 0);
    measured(&report, "update", 1000, "finding");
    assert_eq!(report["mutation_effects"]["findings"], 1);
    assert_eq!(report["mutation_effects"]["visible_findings"], 1);
    let output = replay(
        directory.path(),
        &original,
        &[
            "--density-baseline",
            "baseline.json",
            "--fail-on-new-density",
            "--fail-on-findings",
        ],
    );
    checked(&output, 1);
    counts(&parsed(&output), 1, 0);
}

#[test]
fn density_baseline_reads_and_writes_from_source_and_tool_free_full_and_compact_replay() {
    let (directory, original) = exported(ORIGINAL);
    for path in ["src/lib.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let output = replay(
            directory.path(),
            &original,
            &[
                "--density-baseline",
                "baseline.json",
                "--fail-on-new-density",
                "--report-detail",
                detail,
            ],
        );
        checked(&output, 0);
        let report = parsed(&output);
        counts(&report, 1, 0);
        assert_eq!(report["verification_policy"], "strict-replay");
        assert_eq!(density(&report, "update"), density(&original, "update"));
        assert_replay_export(directory.path(), &original, detail);
    }
}

fn assert_replay_export(directory: &Path, original: &Value, detail: &str) {
    let name = format!("baseline-{detail}.json");
    let output = replay(
        directory,
        original,
        &["--write-density-baseline", &name, "--report-detail", detail],
    );
    checked(&output, 0);
    let output = replay(
        directory,
        original,
        &["--density-baseline", &name, "--fail-on-new-density"],
    );
    checked(&output, 0);
    counts(&parsed(&output), 1, 0);
}

#[test]
fn moved_density_owner_keeps_numeric_baseline_but_new_owner_and_measurement_do_not() {
    let (directory, _) = exported(ORIGINAL);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("\n\n// Move the original without changing its measurement.\n{ORIGINAL}"),
    )
    .unwrap();
    let (output, moved) = requested(directory.path(), BASELINE);
    checked(&output, 0);
    counts(&moved, 1, 0);
    assert_changed_source(
        directory.path(),
        "pub fn update(target:&mut u32,other:&mut u32){*target=1;*other=2;}\n",
        0,
        1,
    );
    assert_changed_source(
        directory.path(),
        &format!("{ORIGINAL}pub fn additional(target:&mut u32){{*target=1;}}\n"),
        1,
        1,
    );
}

fn assert_changed_source(directory: &Path, source: &str, existing: usize, new: usize) {
    std::fs::write(directory.join("src/lib.rs"), source).unwrap();
    let (output, report) = requested(directory, BASELINE);
    checked(&output, 1);
    counts(&report, existing, new);
    measured(&report, "update", 1000, "finding");
}

#[test]
fn changed_density_policy_cannot_reuse_an_existing_numeric_baseline() {
    let (directory, original) = exported(ORIGINAL);
    for policy in [
        POLICY.replace("max_density_milli=500", "max_density_milli=400"),
        format!("{POLICY}\n[side_effects.weights]\nmutation=2\n"),
    ] {
        std::fs::write(directory.path().join("effects.toml"), policy).unwrap();
        let output = replay(directory.path(), &original, BASELINE);
        checked(&output, 1);
        let report = parsed(&output);
        counts(&report, 0, 1);
        assert_eq!(density(&report, "update")["complete"], true);
        assert_eq!(report["mutation_effects"]["findings"], 1);
    }
}

#[test]
fn incomplete_density_export_leaves_no_baseline_file() {
    let directory = workspace("pub fn update(){std::thread::yield_now();}\n", POLICY);
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--effect-density",
            "--effect-config",
            "effects.toml",
            "--write-density-baseline",
            "baseline.json",
        ],
    );
    checked(&output, 2);
    assert!(!directory.path().join("baseline.json").exists());
}

#[test]
fn empty_changed_selection_cannot_hide_incomplete_density_baseline_comparison() {
    let (directory, original) = exported(ORIGINAL);
    select(directory.path(), &[]);
    let output = replay(
        directory.path(),
        &original,
        &[
            "--density-baseline",
            "baseline.json",
            "--fail-on-new-density",
            "--changed-files",
            "selection.json",
            "--max-analysis-functions",
            "0",
        ],
    );
    checked(&output, 2);
    let report = parsed(&output);
    assert_eq!(report["mutation_effects"]["execution"], "budget_exhausted");
    assert_eq!(
        report["density_review"]["baseline"]["coverage"],
        "incomplete"
    );
}

#[test]
fn density_baseline_exports_refuse_overwrite_and_failed_exports_leave_no_file() {
    let (directory, original) = exported(ORIGINAL);
    let path = directory.path().join("baseline.json");
    let before = std::fs::read(&path).unwrap();
    let output = replay(
        directory.path(),
        &original,
        &["--write-density-baseline", "baseline.json"],
    );
    checked(&output, 2);
    assert_eq!(std::fs::read(path).unwrap(), before);
    let output = replay(
        directory.path(),
        &original,
        &[
            "--write-density-baseline",
            "budget.json",
            "--max-analysis-functions",
            "0",
        ],
    );
    checked(&output, 2);
    assert!(!directory.path().join("budget.json").exists());
}

#[test]
fn malformed_density_baseline_is_rejected_before_capture() {
    let directory = workspace(ORIGINAL, POLICY);
    std::fs::write(directory.path().join("baseline.json"), "{broken").unwrap();
    let output = diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap())
            .current_dir(directory.path())
            .args([
                "clones",
                "--effect-density",
                "--density-baseline",
                "baseline.json",
                "--fail-on-new-density",
            ])
            .env("PATH", directory.path().join("no-tools"))
            .env("CARGO", directory.path().join("no-cargo"))
            .env("RSTYLE_DRIVER", directory.path().join("no-driver")),
    )
    .unwrap();
    checked(&output, 2);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("baseline"), "{error}");
    assert!(!error.contains("no-cargo"), "{error}");
}

fn select(directory: &Path, paths: &[&str]) {
    std::fs::write(
        directory.join("selection.json"),
        serde_json::to_vec(&serde_json::json!({
            "schema": "rstyle.changed-files/1", "paths": paths,
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn density_baseline_counts_selected_findings_after_saved_review_visibility() {
    let (directory, original) = exported(ORIGINAL);
    select(directory.path(), &[]);
    let output = replay(
        directory.path(),
        &original,
        &[
            "--density-baseline",
            "baseline.json",
            "--fail-on-new-density",
            "--changed-files",
            "selection.json",
        ],
    );
    checked(&output, 0);
    counts(&parsed(&output), 0, 0);
    assert_hidden_baseline(directory.path(), &original);
}

fn assert_hidden_baseline(directory: &Path, original: &Value) {
    let key = function(original, "update")["key"].as_str().unwrap();
    let output = replay(
        directory,
        original,
        &[
            "--density-baseline",
            "baseline.json",
            "--fail-on-new-density",
            "--review-file",
            "reviews.json",
            "--dismiss-density",
            key,
            "--review-reason",
            "Keep the caller-visible write.",
        ],
    );
    checked(&output, 0);
    let report = parsed(&output);
    counts(&report, 0, 0);
    assert_eq!(report["density_review"]["hidden_findings"], 1);
    measured(&report, "update", 1000, "finding");
}

#[test]
fn density_baseline_flags_require_density_and_whole_scope_export() {
    let directory = workspace(ORIGINAL, POLICY);
    for flags in [
        vec!["--write-density-baseline", "baseline.json"],
        vec!["--density-baseline", "baseline.json"],
        vec!["--effect-density", "--fail-on-new-density"],
        vec![
            "--effect-density",
            "--write-density-baseline",
            "baseline.json",
            "--density-baseline",
            "baseline.json",
        ],
        vec![
            "--effect-density",
            "--write-density-baseline",
            "baseline.json",
            "--changed-files",
            "selection.json",
        ],
    ] {
        let mut arguments = vec!["clones"];
        arguments.extend(flags);
        let output = rstyle(directory.path(), &arguments);
        checked(&output, 2);
        assert!(!directory.path().join("baseline.json").exists());
    }
}

#[test]
fn empty_density_baseline_still_checks_policy_and_exclusion_scope() {
    let (directory, original) = exported("pub fn identity(value:u32)->u32{value}\n");
    let stored: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("baseline.json")).unwrap())
            .unwrap();
    assert!(stored["entries"].as_array().unwrap().is_empty());
    for policy in [
        POLICY.replace("max_density_milli=500", "max_density_milli=1000"),
        format!("exclude=['src/missing.rs']\n{POLICY}"),
    ] {
        std::fs::write(directory.path().join("effects.toml"), policy).unwrap();
        let output = replay(directory.path(), &original, BASELINE);
        checked(&output, 1);
        let report = parsed(&output);
        counts(&report, 0, 0);
        assert_eq!(report["density_review"]["baseline"]["scope_changed"], true);
        assert_eq!(report["mutation_effects"]["findings"], 0);
    }
}

#[test]
fn empty_density_baseline_still_checks_retained_build_context() {
    let (directory, original) = exported("pub fn identity(value:u32)->u32{value}\n");
    let path = directory.path().join("baseline.json");
    let mut stored: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(stored["entries"].as_array().unwrap().is_empty());
    let previous = stored["contexts"][0]["compiler_commit"].as_str().unwrap();
    stored["contexts"][0]["compiler_commit"] = Value::String(format!("{previous}-changed"));
    std::fs::write(path, serde_json::to_vec(&stored).unwrap()).unwrap();
    let output = replay(directory.path(), &original, BASELINE);
    checked(&output, 1);
    let report = parsed(&output);
    counts(&report, 0, 0);
    assert_eq!(report["density_review"]["baseline"]["scope_changed"], true);
    assert_eq!(report["mutation_effects"]["findings"], 0);
}

#[test]
fn retained_density_baseline_requires_completed_numeric_negative_to_resolve() {
    let (directory, _) = exported(ORIGINAL);
    for (source, state, raw_findings) in [
        (
            "pub fn update(target:&mut u32){}\n".to_owned(),
            "resolved",
            0,
        ),
        (
            format!("// rstyle-allow(side_effect_density): deliberate update\n{ORIGINAL}"),
            "unavailable",
            1,
        ),
        (
            "pub fn unrelated(value:u32)->u32{value}\n".to_owned(),
            "unobserved",
            0,
        ),
    ] {
        std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
        let (output, report) = requested(directory.path(), BASELINE);
        checked(&output, 0);
        counts(&report, 0, 0);
        let retained = report["density_review"]["baseline"]["retained"]
            .as_array()
            .unwrap();
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0]["state"], state, "{retained:?}");
        assert_eq!(report["mutation_effects"]["findings"], raw_findings);
    }
}
