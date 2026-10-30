use super::*;

const GUARDED: &str =
    "pub fn nested(value:u32)->u32 { if value>0 { let next=value+1; next+1 } else { 0 } }\n";
const FLAGS: &[&str] = &[
    "--extraction-profile",
    "hir-data-v3",
    "--compiler-advisory",
    "--source-rules",
    "--guard-clauses",
    "--guard-plans",
    "--changed-files",
    "selection.json",
    "--fail-on-findings",
];

fn fixture() -> TempDir {
    let directory = clone_workspace();
    for (path, source) in [
        ("src/lib.rs", "pub mod guarded; pub mod clean;\n"),
        ("src/guarded.rs", GUARDED),
        ("src/clean.rs", "pub fn clean(value:u32)->u32 { value }\n"),
    ] {
        std::fs::write(directory.path().join(path), source).unwrap();
    }
    selection(directory.path(), &["src/clean.rs"]);
    directory
}

fn selection(directory: &Path, paths: &[&str]) {
    std::fs::write(
        directory.join("selection.json"),
        serde_json::to_vec(&serde_json::json!({
            "schema":"rstyle.changed-files/1", "paths":paths,
        }))
        .unwrap(),
    )
    .unwrap();
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

fn capture(directory: &Path, raw: bool) -> (Output, Value) {
    let mut args = FLAGS.to_vec();
    if raw {
        args.push("--compiler-advisory-only");
    }
    clones(directory, &args)
}

fn saved(directory: &Path, original: &Value, extra: &[&str]) -> Output {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command.current_dir(directory).arg("clones");
    if !extra.contains(&"--format") {
        command.args(["--format", "json"]);
    }
    command
        .args(FLAGS)
        .args(["--replay-run", original["run_directory"].as_str().unwrap()])
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    diagnostic_process::output(&mut command).unwrap()
}

fn parsed(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)))
}

fn unchanged(report: &Value, original: &Value) {
    for field in ["findings", "visible_findings", "scope"] {
        assert_eq!(
            report["analysis"][field], original["analysis"][field],
            "{field}"
        );
    }
    let functions = report["analysis"]["functions"].as_array().unwrap();
    let previous = original["analysis"]["functions"].as_array().unwrap();
    assert_eq!(functions.len(), previous.len());
    for (function, prior) in functions.iter().zip(previous) {
        for field in ["key", "assessments", "guard_clause"] {
            assert_eq!(function[field], prior[field], "{field}");
        }
    }
    assert_eq!(report["guard_planning"], original["guard_planning"]);
    assert_eq!(report["edit_coordination"], original["edit_coordination"]);
}

fn selected(directory: &Path, original: &Value, raw: bool) -> Value {
    let mut extra = vec!["--select-guards"];
    if raw {
        extra.push("--compiler-advisory-only");
    }
    let output = saved(directory, original, &extra);
    checked(&output, 0);
    let report = parsed(&output);
    unchanged(&report, original);
    let scope = &report["analysis"]["guard_changed_scope"];
    assert_eq!(scope["coverage"], "complete");
    assert_eq!(scope["visible_findings"], 0);
    assert_eq!(scope["paths"][0]["matched_functions"], 1);
    assert_eq!(scope["paths"][0]["status"], "observed");
    report
}

#[test]
fn guard_file_selection_is_opt_in_and_raw_capture_reuses_the_same_assessments_and_plans() {
    let directory = fixture();
    let (output, combined) = capture(directory.path(), false);
    checked(&output, 1);
    assert!(combined["analysis"].get("guard_changed_scope").is_none());
    let ordinary = selected(directory.path(), &combined, false);
    let (output, raw) = capture(directory.path(), true);
    checked(&output, 1);
    let selected_raw = selected(directory.path(), &raw, true);
    assert_eq!(selected_raw["analysis"], ordinary["analysis"]);
    assert_eq!(selected_raw["guard_planning"], ordinary["guard_planning"]);
    assert_eq!(selected_raw["matching_execution"], "not_requested");
    assert!(selected_raw.get("matching").is_none());
    guard_only_selection(directory.path(), &raw);
}

fn guard_only_selection(directory: &Path, original: &Value) {
    let (output, report) = clones(
        directory,
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-advisory-only",
            "--guard-clauses",
            "--select-guards",
            "--changed-files",
            "selection.json",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--fail-on-findings",
        ],
    );
    checked(&output, 0);
    assert!(report["analysis"].get("changed_scope").is_none());
    assert_eq!(
        report["analysis"]["guard_changed_scope"]["coverage"],
        "complete"
    );
    assert_eq!(report["analysis"]["findings"], 1);
    assert_eq!(report["analysis"]["effective_config"], Value::Null);
}

fn source_free_selections(directory: &Path, original: &Value) {
    for path in ["src/lib.rs", "src/guarded.rs", "src/clean.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.join(path)).unwrap();
    }
    for (paths, code, coverage, findings) in [
        (vec!["src/guarded.rs"], 1, "complete", 1),
        (vec!["src/clean.rs"], 0, "complete", 0),
        (vec!["src/missing.rs"], 2, "incomplete", 0),
    ] {
        selection(directory, &paths);
        for detail in ["full", "compact"] {
            let output = saved(
                directory,
                original,
                &[
                    "--compiler-advisory-only",
                    "--select-guards",
                    "--report-detail",
                    detail,
                ],
            );
            checked(&output, code);
            let report = parsed(&output);
            unchanged(&report, original);
            assert_eq!(
                report["analysis"]["guard_changed_scope"]["coverage"],
                coverage
            );
            assert_eq!(
                report["analysis"]["guard_changed_scope"]["visible_findings"],
                findings
            );
            assert_eq!(report["strict_assessment_execution"], "not_requested");
        }
    }
}

fn guard_text_selection(directory: &Path, original: &Value) {
    selection(directory, &["src/clean.rs"]);
    for (select, count) in [(false, 1), (true, 0)] {
        let mut extra = vec!["--compiler-advisory-only", "--format", "text"];
        if select {
            extra.push("--select-guards");
        }
        let output = saved(directory, original, &extra);
        checked(&output, if select { 0 } else { 1 });
        assert_eq!(stdout(&output).matches("\nguard_clause ").count(), count);
    }
}

#[test]
fn source_and_tool_free_replay_selects_guard_clean_and_unobserved_files_without_changing_plans() {
    let directory = fixture();
    let (output, original) = capture(directory.path(), true);
    checked(&output, 1);
    source_free_selections(directory.path(), &original);
    guard_text_selection(directory.path(), &original);
}

#[test]
fn multiple_closure_opportunities_count_once_and_empty_selection_cannot_hide_incomplete_coverage() {
    let directory = fixture();
    std::fs::write(directory.path().join("src/guarded.rs"),
        "pub fn outer(value:u32)->u32 { let first=|x:u32| { if x>0 { let next=x+1; next+1 } else { 0 } }; let second=|y:u32| { if y>0 { let next=y+2; next+2 } else { 0 } }; first(value)+second(value) }\n").unwrap();
    selection(directory.path(), &["src/guarded.rs"]);
    let (output, original) = capture(directory.path(), true);
    checked(&output, 2);
    for paths in [vec!["src/guarded.rs"], vec![]] {
        selection(directory.path(), &paths);
        let output = saved(
            directory.path(),
            &original,
            &["--compiler-advisory-only", "--select-guards"],
        );
        checked(&output, 2);
        let report = parsed(&output);
        unchanged(&report, &original);
        assert_eq!(report["analysis"]["findings"], 1);
        assert_eq!(
            report["guard_planning"]["plans"].as_array().unwrap().len(),
            2
        );
        assert_eq!(
            report["analysis"]["guard_changed_scope"]["visible_findings"],
            usize::from(!paths.is_empty())
        );
        assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
    }
}

#[test]
fn explicit_guard_file_selection_requires_both_guard_analysis_and_a_manifest() {
    let directory = fixture();
    for missing in ["--guard-clauses", "--changed-files"] {
        let mut args = vec!["clones", "--select-guards"];
        if missing == "--changed-files" {
            args.push("--guard-clauses");
        } else {
            args.extend(["--source-rules", "--changed-files", "selection.json"]);
        }
        let output = rstyle(directory.path(), &args);
        checked(&output, 2);
        assert!(String::from_utf8_lossy(&output.stderr).contains(missing));
    }
}
