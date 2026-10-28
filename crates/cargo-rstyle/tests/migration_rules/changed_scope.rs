use super::*;

fn workspace() -> TempDir {
    let directory = clone_workspace();
    for (path, source) in [
        ("src/lib.rs", "pub mod clean; pub mod bad;\n"),
        (
            "src/clean.rs",
            "pub fn clean(value: u32) -> u32 { value }\n",
        ),
        (
            "src/bad.rs",
            "pub fn many(a: u32, b: u32) -> u32 { a + b }\n",
        ),
        (
            "analysis.toml",
            "[fn_length]\nmax_lines=100\n[fn_params]\nmax_params=1\n",
        ),
        (
            "Cargo.lock",
            "version = 4\n\n[[package]]\nname = \"clone_fixture\"\nversion = \"0.1.0\"\n",
        ),
    ] {
        std::fs::write(directory.path().join(path), source).unwrap();
    }
    select(directory.path(), &["src/clean.rs"]);
    directory
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

fn requested(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--locked",
        "--extraction-profile",
        "hir-data-v3",
        "--source-rules",
        "--analysis-config",
        "analysis.toml",
        "--changed-files",
        "selection.json",
    ];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn scope(report: &Value) -> &Value {
    &report["analysis"]["changed_scope"]
}

fn selected_path<'a>(report: &'a Value, path: &str) -> &'a Value {
    scope(report)["paths"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == path)
        .unwrap()
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
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
        ])
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)));
    (output, report)
}

#[test]
fn changed_clean_file_keeps_all_owner_assessments_but_filters_source_finding_gate_and_text() {
    let directory = workspace();
    let (output, report) = requested(
        directory.path(),
        &["--fail-on-findings", "--parameter-plans"],
    );
    assert!(output.status.success(), "{}", report["analysis"]);
    assert_eq!(report["analysis"]["visible_findings"], 1);
    assert_eq!(report["parameter_planning"]["findings"], 1);
    assert_eq!(scope(&report)["coverage"], "complete");
    assert_eq!(scope(&report)["visible_findings"], 0);
    assert_eq!(assessed(&report, "clean::clean")["changed_file"], true);
    assert_eq!(assessed(&report, "bad::many")["changed_file"], false);
    assert_eq!(
        assessed(&report, "bad::many")["assessments"][1]["outcome"],
        "finding"
    );
    let selected = selected_path(&report, "src/clean.rs");
    assert_eq!(selected["matched_functions"], 1);
    assert_eq!(selected["unavailable_functions"], 0);
    assert_eq!(selected["status"], "observed");
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--locked",
            "--extraction-profile",
            "hir-data-v3",
            "--source-rules",
            "--analysis-config",
            "analysis.toml",
            "--changed-files",
            "selection.json",
        ],
    );
    assert!(output.status.success());
    assert!(!stdout(&output).contains("\nfn_params "));
}

#[test]
fn selected_violation_fails_and_source_free_replay_can_select_another_manifest() {
    let directory = workspace();
    select(directory.path(), &["src/bad.rs"]);
    let (output, original) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{}", original["analysis"]);
    assert_eq!(scope(&original)["visible_findings"], 1);
    for path in ["Cargo.toml", "src/lib.rs", "src/clean.rs", "src/bad.rs"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let (output, report) = replay(
            directory.path(),
            &original,
            &[
                "--changed-files",
                "selection.json",
                "--report-detail",
                detail,
                "--fail-on-findings",
            ],
        );
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(report["verification_policy"], "strict-replay");
        assert_eq!(report["analysis"], original["analysis"]);
    }
    select(directory.path(), &["src/clean.rs"]);
    let (output, changed) = replay(
        directory.path(),
        &original,
        &["--changed-files", "selection.json", "--fail-on-findings"],
    );
    assert!(output.status.success(), "{}", changed["analysis"]);
    assert_eq!(scope(&changed)["visible_findings"], 0);
    assert_eq!(
        assessed(&changed, "bad::many")["assessments"],
        assessed(&original, "bad::many")["assessments"]
    );
    let (_, unrequested) = replay(directory.path(), &original, &[]);
    assert!(unrequested["analysis"].get("changed_scope").is_none());
    assert!(
        assessed(&unrequested, "bad::many")
            .get("changed_file")
            .is_none()
    );
}

#[test]
fn empty_and_unknown_selections_do_not_hide_global_budget_incompleteness() {
    let directory = workspace();
    let (output, original) = requested(directory.path(), &[]);
    assert!(output.status.success());
    let (output, limited) = replay(
        directory.path(),
        &original,
        &[
            "--changed-files",
            "selection.json",
            "--max-analysis-functions",
            "0",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(limited["analysis"]["execution"], "budget_exhausted");
    select(directory.path(), &[]);
    let flags = ["--changed-files", "selection.json", "--fail-on-findings"];
    let (output, empty) = replay(directory.path(), &original, &flags);
    assert!(output.status.success());
    assert_eq!(scope(&empty)["coverage"], "complete");
    assert_eq!(scope(&empty)["visible_findings"], 0);
    assert!(scope(&empty)["paths"].as_array().unwrap().is_empty());
    let (output, limited) = replay(
        directory.path(),
        &original,
        &[
            "--changed-files",
            "selection.json",
            "--max-analysis-functions",
            "0",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(limited["analysis"]["execution"], "budget_exhausted");
    select(directory.path(), &["src/missing.rs"]);
    let (output, unknown) = replay(directory.path(), &original, &flags);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(scope(&unknown)["coverage"], "incomplete");
    let path = selected_path(&unknown, "src/missing.rs");
    assert_eq!(path["status"], "unobserved");
    assert_eq!(path["matched_functions"], 0);
    assert_eq!(path["coverage"], "incomplete");
}

#[test]
fn changed_scope_retains_suppression_and_exclusion_evidence() {
    let directory = workspace();
    std::fs::write(directory.path().join("src/clean.rs"),
        "// rstyle-allow(fn_params): generated adapter\npub fn clean(a: u32, b: u32) -> u32 { a + b }\n").unwrap();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "exclude=[\"src/bad.rs\"]\n[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    select(directory.path(), &["src/clean.rs", "src/bad.rs"]);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", report["analysis"]);
    assert_eq!(scope(&report)["coverage"], "complete");
    assert_eq!(scope(&report)["visible_findings"], 0);
    assert_eq!(
        assessed(&report, "clean::clean")["source_suppression"]["status"],
        "allowed"
    );
    assert_eq!(
        assessed(&report, "bad::many")["source_exclusion"]["status"],
        "excluded"
    );
    for path in ["src/clean.rs", "src/bad.rs"] {
        assert_eq!(selected_path(&report, path)["matched_functions"], 1);
        assert_eq!(selected_path(&report, path)["visible_findings"], 0);
    }
}

#[test]
fn exact_paths_distinguish_same_named_files_and_all_compiled_owner_occurrences() {
    let directory = workspace();
    for name in ["left", "right"] {
        std::fs::create_dir(directory.path().join(format!("src/{name}"))).unwrap();
        std::fs::write(
            directory.path().join(format!("src/{name}/same.rs")),
            "pub fn same(a: u32, b: u32) -> u32 { a + b }\n",
        )
        .unwrap();
    }
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub mod left { pub mod same; } pub mod right { pub mod same; }\n",
    )
    .unwrap();
    select(directory.path(), &["src/left/same.rs"]);
    let (output, report) = requested(directory.path(), &["--all-targets", "--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(2), "{}", report["analysis"]);
    assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
    assert_eq!(scope(&report)["coverage"], "complete");
    assert_eq!(
        selected_path(&report, "src/left/same.rs")["coverage"],
        "complete"
    );
    assert_eq!(
        selected_path(&report, "src/left/same.rs")["matched_functions"],
        2
    );
    assert_eq!(scope(&report)["visible_findings"], 2);
    let owners = report["analysis"]["functions"].as_array().unwrap();
    assert_eq!(
        owners
            .iter()
            .filter(|owner| owner["changed_file"] == true)
            .count(),
        2
    );
    assert_eq!(
        owners
            .iter()
            .filter(|owner| owner["name"] == "right::same::same" && owner["changed_file"] == false)
            .count(),
        2
    );
}

#[test]
fn changed_source_filter_does_not_suppress_guard_findings_in_unchanged_files() {
    let directory = workspace();
    std::fs::write(directory.path().join("src/bad.rs"),
        "pub fn guarded(value: u32) -> u32 { if value > 0 { let next = value + 1; next + 1 } else { 0 } }\n").unwrap();
    select(directory.path(), &[]);
    let (output, report) = requested(directory.path(), &["--guard-clauses", "--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{}", report["analysis"]);
    assert_eq!(scope(&report)["visible_findings"], 0);
    assert_eq!(assessed(&report, "bad::guarded")["changed_file"], false);
    assert_eq!(
        assessed(&report, "bad::guarded")["guard_clause"]["outcome"],
        "finding"
    );
}

#[test]
fn changed_file_selection_filters_length_and_spacing_findings_together() {
    let directory = workspace();
    std::fs::write(directory.path().join("src/bad.rs"),
        "pub fn many(value: u32) -> u32 {\n    let a = value + 1;\n    let b = a + 1;\n    let c = b + 1;\n    let d = c + 1;\n    d\n}\n").unwrap();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=5\n[fn_params]\nmax_params=10\n[fn_spacing]\nmin_statements=5\n",
    )
    .unwrap();
    select(directory.path(), &["src/bad.rs"]);
    let (output, original) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{}", original["analysis"]);
    assert_eq!(scope(&original)["coverage"], "complete");
    assert_eq!(scope(&original)["visible_findings"], 2);
    assert_eq!(original["analysis"]["findings"], 2);
    let assessments = assessed(&original, "bad::many")["assessments"]
        .as_array()
        .unwrap();
    for (rule, actual) in [("fn_length", 7), ("fn_spacing", 5)] {
        let assessment = assessments
            .iter()
            .find(|assessment| assessment["rule_id"] == rule)
            .unwrap();
        assert_eq!(assessment["outcome"], "finding");
        assert_eq!(assessment["actual"], actual);
    }
    select(directory.path(), &["src/clean.rs"]);
    let (output, clean) = replay(
        directory.path(),
        &original,
        &["--changed-files", "selection.json", "--fail-on-findings"],
    );
    assert!(output.status.success(), "{}", clean["analysis"]);
    assert_eq!(scope(&clean)["coverage"], "complete");
    assert_eq!(scope(&clean)["visible_findings"], 0);
    assert_eq!(clean["analysis"]["findings"], 2);
    assert_eq!(clean["analysis"]["visible_findings"], 2);
    assert_eq!(
        assessed(&clean, "bad::many")["assessments"],
        assessed(&original, "bad::many")["assessments"]
    );
}

#[test]
fn malformed_changed_manifest_and_missing_dependency_are_rejected_before_capture() {
    let directory = workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn broken( {").unwrap();
    for manifest in [
        "{\"schema\":\"wrong\",\"paths\":[]}",
        "{\"schema\":\"rstyle.changed-files/1\",\"paths\":[\"../outside.rs\"]}",
        "{\"schema\":\"rstyle.changed-files/1\",\"paths\":[],\"trust\":true}",
    ] {
        std::fs::write(directory.path().join("selection.json"), manifest).unwrap();
        let output = rstyle(
            directory.path(),
            &[
                "clones",
                "--source-rules",
                "--changed-files",
                "selection.json",
            ],
        );
        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("selection.json"), "{stderr}");
        assert!(!stderr.contains("could not compile"), "{stderr}");
    }
    let output = rstyle(
        directory.path(),
        &["clones", "--changed-files", "selection.json"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--source-rules"));
}
