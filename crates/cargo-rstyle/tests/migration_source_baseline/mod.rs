use super::*;
mod external_methods;
mod generated_applicability;

const ORIGINAL: &str = "pub fn many(a: u32, b: u32) -> u32 {\n    let first = a + b;\n    let second = first + 1;\n    let third = second + 1;\n    let fourth = third + 1;\n    fourth + 1\n}\n";
const POLICY: &str = "[fn_length]\nmax_lines=2\n[fn_params]\nmax_params=1\ncount_self=false\n[fn_spacing]\nmin_statements=5\n";
const BASELINE: &[&str] = &["--source-baseline", "baseline.json", "--fail-on-new-source"];

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    for (path, bytes) in [
        ("src/lib.rs", source),
        ("analysis.toml", POLICY),
        (
            "Cargo.lock",
            "version = 4\n\n[[package]]\nname = \"clone_fixture\"\nversion = \"0.1.0\"\n",
        ),
    ] {
        std::fs::write(directory.path().join(path), bytes).unwrap();
    }
    directory
}

fn requested(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut arguments = vec![
        "--locked",
        "--extraction-profile",
        "hir-data-v3",
        "--source-rules",
        "--analysis-config",
        "analysis.toml",
    ];
    arguments.extend_from_slice(extra);
    clones(directory, &arguments)
}

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
    let baseline = &report["source_baseline"];
    assert_eq!(baseline["schema"], "rstyle.source-baseline-report/1");
    assert_eq!(baseline["coverage"], "complete", "{baseline}");
    assert_eq!(baseline["existing_findings"], existing, "{baseline}");
    assert_eq!(baseline["new_findings"], new, "{baseline}");
}

fn exported(source: &str) -> (TempDir, Value) {
    let directory = workspace(source);
    let (output, report) = requested(
        directory.path(),
        &["--write-source-baseline", "baseline.json"],
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
        "--source-rules",
        "--analysis-config",
        "analysis.toml",
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

fn stored(directory: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(directory.join("baseline.json")).unwrap()).unwrap()
}

fn select(directory: &Path, paths: &[&str]) {
    std::fs::write(
        directory.join("selection.json"),
        serde_json::to_vec(&serde_json::json!({"schema":"rstyle.changed-files/1", "paths":paths}))
            .unwrap(),
    )
    .unwrap();
}

#[test]
fn source_baseline_preserves_three_families_on_one_owner_and_ordinary_findings_gate() {
    let (directory, original) = exported(ORIGINAL);
    let saved = stored(directory.path());
    assert_eq!(saved["schema"], "rstyle.source-baseline/1");
    let mut families: Vec<_> = saved["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["rule"].as_str().unwrap())
        .collect();
    families.sort_unstable();
    assert_eq!(families, ["length", "parameter", "spacing"]);
    let output = replay(directory.path(), &original, BASELINE);
    checked(&output, 0);
    let report = parsed(&output);
    counts(&report, 3, 0);
    assert_eq!(report["analysis"]["findings"], 3);
    assert_eq!(report["analysis"]["visible_findings"], 3);
    let output = replay(
        directory.path(),
        &original,
        &[
            "--source-baseline",
            "baseline.json",
            "--fail-on-new-source",
            "--fail-on-findings",
        ],
    );
    checked(&output, 1);
    counts(&parsed(&output), 3, 0);
}

#[test]
fn source_baseline_recomputes_from_source_and_tool_free_full_and_compact_replay() {
    let (directory, original) = exported(ORIGINAL);
    for path in ["src/lib.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let output = replay(
            directory.path(),
            &original,
            &[
                "--source-baseline",
                "baseline.json",
                "--fail-on-new-source",
                "--report-detail",
                detail,
            ],
        );
        checked(&output, 0);
        let report = parsed(&output);
        counts(&report, 3, 0);
        assert_eq!(report["verification_policy"], "strict-replay");
        assert_eq!(report["analysis"], original["analysis"]);
        assert_replay_export(directory.path(), &original, detail);
    }
}

fn assert_replay_export(directory: &Path, original: &Value, detail: &str) {
    let name = format!("baseline-{detail}.json");
    let output = replay(
        directory,
        original,
        &["--write-source-baseline", &name, "--report-detail", detail],
    );
    checked(&output, 0);
    let output = replay(
        directory,
        original,
        &["--source-baseline", &name, "--fail-on-new-source"],
    );
    checked(&output, 0);
    counts(&parsed(&output), 3, 0);
}

#[test]
fn moved_source_baseline_owner_remains_existing_but_changed_and_new_measurements_fail() {
    let (directory, _) = exported(ORIGINAL);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("\n\n// Move the owner.\n{ORIGINAL}"),
    )
    .unwrap();
    let (output, moved) = requested(directory.path(), BASELINE);
    checked(&output, 0);
    counts(&moved, 3, 0);
    let changed = ORIGINAL.replace("b: u32)", "b: u32, c: u32)");
    std::fs::write(directory.path().join("src/lib.rs"), changed).unwrap();
    let (output, report) = requested(directory.path(), BASELINE);
    checked(&output, 1);
    counts(&report, 2, 1);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{ORIGINAL}{}", ORIGINAL.replace("many(", "additional(")),
    )
    .unwrap();
    let (output, report) = requested(directory.path(), BASELINE);
    checked(&output, 1);
    counts(&report, 3, 3);
}

#[test]
fn changed_source_baseline_policy_is_not_an_existing_finding() {
    let (directory, original) = exported(ORIGINAL);
    std::fs::write(
        directory.path().join("analysis.toml"),
        POLICY.replace("max_params=1", "max_params=0"),
    )
    .unwrap();
    let output = replay(directory.path(), &original, BASELINE);
    checked(&output, 1);
    let report = parsed(&output);
    counts(&report, 2, 1);
    assert_eq!(report["source_baseline"]["scope_changed"], true);
    assert_eq!(report["analysis"]["findings"], 3);
}

#[test]
fn empty_source_baseline_checks_policy_exclusion_and_disabled_spacing_scope() {
    let (directory, original) = exported("pub fn identity(value:u32)->u32{value}\n");
    assert!(
        stored(directory.path())["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for policy in [
        POLICY.replace("max_params=1", "max_params=2"),
        POLICY.replace("max_lines=2", "max_lines=3"),
        POLICY.replace("[fn_spacing]\nmin_statements=5\n", ""),
        POLICY.replace("min_statements=5", "min_statements=0"),
        format!("exclude=['src/missing.rs']\n{POLICY}"),
    ] {
        std::fs::write(directory.path().join("analysis.toml"), policy).unwrap();
        let output = replay(directory.path(), &original, BASELINE);
        checked(&output, 1);
        let report = parsed(&output);
        counts(&report, 0, 0);
        assert_eq!(report["source_baseline"]["scope_changed"], true);
        assert_eq!(report["analysis"]["findings"], 0);
    }
}

#[test]
fn empty_source_baseline_checks_retained_build_context() {
    let (directory, original) = exported("pub fn identity(value:u32)->u32{value}\n");
    let mut saved = stored(directory.path());
    let previous = saved["contexts"][0]["compiler_commit"].as_str().unwrap();
    saved["contexts"][0]["compiler_commit"] = Value::String(format!("{previous}-changed"));
    std::fs::write(
        directory.path().join("baseline.json"),
        serde_json::to_vec(&saved).unwrap(),
    )
    .unwrap();
    let output = replay(directory.path(), &original, BASELINE);
    checked(&output, 1);
    let report = parsed(&output);
    counts(&report, 0, 0);
    assert_eq!(report["source_baseline"]["scope_changed"], true);
}

#[test]
fn source_baseline_counts_and_retained_states_follow_changed_selection() {
    let (directory, original) = exported(ORIGINAL);
    select(directory.path(), &[]);
    let output = replay(
        directory.path(),
        &original,
        &[
            "--source-baseline",
            "baseline.json",
            "--fail-on-new-source",
            "--changed-files",
            "selection.json",
        ],
    );
    checked(&output, 0);
    let report = parsed(&output);
    counts(&report, 0, 0);
    assert!(
        report["source_baseline"]["retained"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["state"] == "unselected")
    );
}

#[test]
fn source_baseline_counts_follow_saved_review_visibility() {
    let (directory, original) = exported(ORIGINAL);
    let key = original["analysis"]["functions"][0]["key"]
        .as_str()
        .unwrap();
    let output = replay(
        directory.path(),
        &original,
        &[
            "--source-baseline",
            "baseline.json",
            "--fail-on-new-source",
            "--review-file",
            "reviews.json",
            "--dismiss-parameter",
            key,
            "--review-reason",
            "Keep this public boundary.",
        ],
    );
    checked(&output, 0);
    let report = parsed(&output);
    counts(&report, 2, 0);
    assert_eq!(report["parameter_review"]["hidden_findings"], 1);
    assert_eq!(report["analysis"]["findings"], 3);
    let rows = report["source_baseline"]["current"].as_array().unwrap();
    assert_eq!(
        rows.iter()
            .filter(|row| row["review_hidden"] == true)
            .count(),
        1
    );
}

#[test]
fn retained_source_baseline_requires_numeric_negative_to_resolve() {
    let (directory, _) = exported(ORIGINAL);
    for (source, state, findings) in [
        ("pub fn many(a:u32)->u32{a}\n".to_owned(), "resolved", 0),
        (
            format!(
                "// rstyle-allow(fn_params, fn_length, fn_spacing): deliberate adapter\n{ORIGINAL}"
            ),
            "unavailable",
            3,
        ),
        (
            "// rstyle-allow(fn_params, fn_length, fn_spacing): deliberate adapter\npub fn many(a:u32)->u32{a}\n".to_owned(),
            "unavailable",
            0,
        ),
        (
            "pub fn unrelated(a:u32)->u32{a}\n".to_owned(),
            "unobserved",
            0,
        ),
    ] {
        std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
        let (output, report) = requested(directory.path(), BASELINE);
        checked(&output, 0);
        counts(&report, 0, 0);
        let retained = report["source_baseline"]["retained"].as_array().unwrap();
        assert_eq!(retained.len(), 3);
        for entry in retained {
            assert_eq!(entry["state"], state, "{entry}");
        }
        assert_eq!(report["analysis"]["findings"], findings);
    }
}

#[test]
fn source_baseline_exports_refuse_overwrite_and_budget_failures_leave_no_artifact() {
    let (directory, original) = exported(ORIGINAL);
    let path = directory.path().join("baseline.json");
    let before = std::fs::read(&path).unwrap();
    let output = replay(
        directory.path(),
        &original,
        &["--write-source-baseline", "baseline.json"],
    );
    checked(&output, 2);
    assert_eq!(std::fs::read(path).unwrap(), before);
    let output = replay(
        directory.path(),
        &original,
        &[
            "--write-source-baseline",
            "budget.json",
            "--max-analysis-functions",
            "0",
        ],
    );
    checked(&output, 2);
    assert!(!directory.path().join("budget.json").exists());
}

#[test]
fn empty_selection_cannot_hide_incomplete_source_baseline_comparison() {
    let (directory, original) = exported(ORIGINAL);
    select(directory.path(), &[]);
    let output = replay(
        directory.path(),
        &original,
        &[
            "--source-baseline",
            "baseline.json",
            "--fail-on-new-source",
            "--changed-files",
            "selection.json",
            "--max-analysis-functions",
            "0",
        ],
    );
    checked(&output, 2);
    let report = parsed(&output);
    assert_eq!(report["analysis"]["execution"], "budget_exhausted");
    assert_eq!(report["source_baseline"]["coverage"], "incomplete");
}

#[test]
fn incomplete_source_ownership_and_invalid_suppression_cannot_export_baseline() {
    let directory = workspace(ORIGINAL);
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--source-rules",
            "--write-source-baseline",
            "missing.json",
        ],
    );
    checked(&output, 2);
    assert!(!directory.path().join("missing.json").exists());
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("// rstyle-allow(fn_params)\n{ORIGINAL}"),
    )
    .unwrap();
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--source-rules",
            "--analysis-config",
            "analysis.toml",
            "--write-source-baseline",
            "invalid.json",
        ],
    );
    checked(&output, 2);
    assert!(!directory.path().join("invalid.json").exists());
}

#[test]
fn malformed_source_baseline_rejects_before_capture() {
    let directory = workspace(ORIGINAL);
    std::fs::write(directory.path().join("baseline.json"), "{broken").unwrap();
    let output = diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap())
            .current_dir(directory.path())
            .args([
                "clones",
                "--source-rules",
                "--source-baseline",
                "baseline.json",
                "--fail-on-new-source",
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

#[test]
fn source_baseline_flags_require_source_rules_and_whole_scope_export() {
    let directory = workspace(ORIGINAL);
    for flags in [
        vec!["--write-source-baseline", "baseline.json"],
        vec!["--source-baseline", "baseline.json"],
        vec!["--source-rules", "--fail-on-new-source"],
        vec![
            "--source-rules",
            "--write-source-baseline",
            "baseline.json",
            "--source-baseline",
            "baseline.json",
        ],
        vec![
            "--source-rules",
            "--write-source-baseline",
            "baseline.json",
            "--changed-files",
            "selection.json",
        ],
    ] {
        let mut arguments = vec!["clones"];
        arguments.extend(flags);
        checked(&rstyle(directory.path(), &arguments), 2);
        assert!(!directory.path().join("baseline.json").exists());
    }
}
