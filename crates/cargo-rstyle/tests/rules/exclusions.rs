use super::*;

const MEASURED: &str = "pub fn same(a: u8) -> u8 { a }\n";

fn workspace(source: &str, exclude: Option<&str>) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    policy(directory.path(), exclude);
    directory
}

fn policy(directory: &Path, exclude: Option<&str>) {
    let prefix = exclude.map_or(String::new(), |value| format!("exclude={value}\n"));
    std::fs::write(
        directory.join("analysis.toml"),
        format!("{prefix}[fn_length]\nmax_lines=0\n[fn_params]\nmax_params=0\ncount_self=false\n"),
    )
    .unwrap();
}

fn capture(directory: &Path) -> Value {
    let (output, report) = source_report(directory, &[]);
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        report
    );
    report
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let executable = cargo_bin("cargo-rstyle").canonicalize().unwrap();
    let mut command = Command::new(executable);
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

fn excluded(function: &Value) {
    assert_eq!(function["source_exclusion"]["status"], "excluded");
    assert!(function["source_suppression"].is_null());
    let assessments = function["assessments"].as_array().unwrap();
    assert_eq!(assessments.len(), 2);
    for assessment in assessments {
        assert_eq!(assessment["outcome"], "not_evaluated");
        assert_eq!(assessment["availability"], "not_applicable");
        assert_eq!(assessment["coverage"], "complete");
        assert!(assessment["actual"].is_null());
    }
}

fn complete_exclusions(report: &Value) {
    let scope = &report["analysis"]["scope"];
    assert_eq!(scope["coverage"], "complete");
    assert_eq!(scope["excluded_source_functions"], 1);
    assert_eq!(scope["excluded_rules"], 2);
    assert_eq!(scope["unavailable_source_exclusions"], 0);
    assert_eq!(scope["unavailable_rules"], 0);
}

#[test]
fn workspace_relative_globs_select_paths_without_hiding_namesake_owners() {
    let directory = workspace(
        "pub mod excluded;\npub mod kept;\n",
        Some("[\"src/excluded.rs\"]"),
    );
    for name in ["excluded", "kept"] {
        std::fs::write(directory.path().join(format!("src/{name}.rs")), MEASURED).unwrap();
    }
    let report = capture(directory.path());
    assert_eq!(report["analysis"]["functions"].as_array().unwrap().len(), 2);
    excluded(assessed(&report, "excluded::same"));
    let kept = assessed(&report, "kept::same");
    assert_eq!(kept["source_exclusion"]["status"], "included");
    assert_eq!(kept["assessments"][0]["outcome"], "finding");
    assert_eq!(kept["assessments"][1]["outcome"], "finding");
    assert_eq!(report["analysis"]["findings"], 2);
    assert_eq!(report["analysis"]["scope"]["evaluated_rules"], 2);
    complete_exclusions(&report);
}

#[test]
fn default_target_exclusion_can_be_disabled_from_a_saved_generated_source() {
    let directory = workspace("", None);
    let generated = directory.path().join("target/generated");
    std::fs::create_dir_all(&generated).unwrap();
    let source = generated.join("source.rs");
    std::fs::write(&source, MEASURED).unwrap();
    // Use the same resolved workspace spelling as Cargo, including macOS /private/var.
    let source = source.canonicalize().unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("#[path = {:?}]\npub mod generated;\n", source),
    )
    .unwrap();
    let original = capture(directory.path());
    excluded(assessed(&original, "generated::same"));
    complete_exclusions(&original);
    std::fs::remove_file(source).unwrap();
    delete_original(directory.path());
    assert_disabled_policy_replays(directory.path(), &original, "generated::same");
}

#[test]
fn configured_exclusions_are_reevaluated_without_original_workspace_or_tools() {
    let directory = workspace(MEASURED, Some("[\"src/lib.rs\"]"));
    let original = capture(directory.path());
    excluded(assessed(&original, "same"));
    delete_original(directory.path());
    let (output, frozen) = replay(directory.path(), &original, &["--fail-on-findings"]);
    assert!(output.status.success(), "{frozen}");
    assert_eq!(original["analysis"], frozen["analysis"]);
    assert_disabled_policy_replays(directory.path(), &original, "same");
}

fn delete_original(directory: &Path) {
    for file in ["Cargo.toml", "src/lib.rs"] {
        std::fs::remove_file(directory.join(file)).unwrap();
    }
}

fn assert_disabled_policy_replays(directory: &Path, original: &Value, name: &str) {
    policy(directory, Some("[]"));
    let (output, included) = replay(directory, original, &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{included}");
    assert_eq!(included["verification_policy"], "strict-replay");
    assert_eq!(included["analysis"]["findings"], 2);
    assert_eq!(included["analysis"]["scope"]["coverage"], "complete");
    assert_eq!(
        included["analysis"]["scope"]["excluded_source_functions"],
        0
    );
    assert_eq!(included["analysis"]["scope"]["excluded_rules"], 0);
    assert_eq!(
        assessed(&included, name)["source_exclusion"]["status"],
        "included"
    );
}

#[test]
fn invalid_exclusion_glob_is_rejected_before_the_broken_build_runs() {
    let directory = workspace("pub fn broken( {", Some("[\"[\"]"));
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--source-rules",
            "--analysis-config",
            "analysis.toml",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("exclude"), "{error}");
    assert!(!error.contains("could not compile"), "{error}");
}

#[test]
fn excluded_invalid_comments_do_not_hide_independent_guard_findings() {
    let directory = workspace(
        "// rstyle-allow(fn_length):\n\
         pub fn nested(value: u32) -> u32 { if value > 0 { let next = value + 1; next + 1 } else { 0 } }\n",
        Some("[\"src/lib.rs\"]"),
    );
    let (output, report) =
        source_report(directory.path(), &["--guard-clauses", "--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{report}");
    let function = assessed(&report, "nested");
    excluded(function);
    assert_eq!(function["guard_clause"]["outcome"], "finding");
    assert_eq!(report["analysis"]["findings"], 1);
    assert_eq!(report["analysis"]["visible_findings"], 1);
    assert_eq!(report["analysis"]["scope"]["evaluated_rules"], 1);
    complete_exclusions(&report);
}

#[test]
fn complete_excluded_runs_stay_clean_in_compact_reports_but_budgets_stay_incomplete() {
    let directory = workspace(MEASURED, Some("[\"src/**\"]"));
    let full = capture(directory.path());
    complete_exclusions(&full);
    assert_eq!(full["analysis"]["findings"], 0);
    assert_eq!(full["analysis"]["scope"]["evaluated_rules"], 0);
    let (output, compact) = replay(
        directory.path(),
        &full,
        &["--report-detail", "compact", "--fail-on-findings"],
    );
    assert!(output.status.success(), "{compact}");
    assert_eq!(full["analysis"], compact["analysis"]);
    let (output, budget) = replay(directory.path(), &full, &["--max-analysis-functions", "0"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(budget["analysis"]["execution"], "budget_exhausted");
    let scope = &budget["analysis"]["scope"];
    assert_eq!(scope["coverage"], "incomplete");
    assert_eq!(scope["excluded_source_functions"], 0);
    assert_eq!(scope["excluded_rules"], 0);
    assert_eq!(scope["unavailable_source_exclusions"], 1);
    assert_eq!(
        assessed(&budget, "same")["source_exclusion"]["status"],
        "unavailable"
    );
    assert_eq!(
        assessed(&budget, "same")["assessments"][0]["availability"],
        "unknown"
    );
}

#[test]
fn workspace_globs_do_not_apply_to_checked_source_outside_original_workspace() {
    let external = tempfile::tempdir().unwrap();
    let path = external.path().join("external.rs");
    std::fs::write(&path, MEASURED).unwrap();
    let source = format!("#[path={:?}]\npub mod external;\n", path.to_str().unwrap());
    let directory = workspace(&source, Some("[\"**\"]"));
    let report = capture(directory.path());
    let function = assessed(&report, "external::same");
    assert_eq!(function["source_exclusion"]["status"], "not_applicable");
    assert_eq!(function["assessments"][0]["outcome"], "finding");
    assert_eq!(function["assessments"][1]["outcome"], "finding");
    assert_eq!(report["analysis"]["findings"], 2);
    let scope = &report["analysis"]["scope"];
    assert_eq!(scope["coverage"], "complete");
    assert_eq!(scope["excluded_source_functions"], 0);
    assert_eq!(scope["excluded_rules"], 0);
    assert_eq!(scope["unavailable_source_exclusions"], 0);
}
