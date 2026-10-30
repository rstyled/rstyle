use super::*;

fn workspace() -> TempDir {
    let directory = super::workspace("pub mod clean; pub mod bad;\n", POLICY);
    for (path, source) in [
        ("src/clean.rs", "pub fn clean(value:u32)->u32{value}\n"),
        ("src/bad.rs", "pub fn bad(target:&mut u32){*target=1;}\n"),
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
    let mut arguments = vec!["--changed-files", "selection.json"];
    arguments.extend_from_slice(extra);
    super::requested(directory, &arguments)
}

fn scope(report: &Value) -> &Value {
    &report["mutation_effects"]["changed_scope"]
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
    let mut command = replay_command(directory, original, "json");
    let output = diagnostic_process::output(command.args(extra)).unwrap();
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)));
    (output, report)
}

fn replay_command(directory: &Path, original: &Value, format: &str) -> Command {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "clones",
            "--format",
            format,
            "--extraction-profile",
            "hir-data-v3",
            "--effect-density",
            "--effect-config",
            "effects.toml",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    command
}

#[test]
fn changed_clean_file_preserves_density_measurements_and_filters_finding_gate_and_text() {
    let directory = workspace();
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(report["analysis"]["execution"], "not_requested");
    assert_eq!(report["mutation_effects"]["findings"], 1);
    assert_eq!(report["mutation_effects"]["visible_findings"], 1);
    assert_eq!(scope(&report)["coverage"], "complete");
    assert_eq!(scope(&report)["visible_findings"], 0);
    assert_eq!(function(&report, "clean::clean")["changed_file"], true);
    assert_eq!(function(&report, "bad::bad")["changed_file"], false);
    measured(&report, "bad::bad", 1000, "finding");
    let selected = selected_path(&report, "src/clean.rs");
    assert_eq!(selected["matched_functions"], 1);
    assert_eq!(selected["unavailable_functions"], 0);
    assert_eq!(selected["status"], "observed");
    let output =
        diagnostic_process::output(replay_command(directory.path(), &report, "text").args([
            "--changed-files",
            "selection.json",
            "--fail-on-findings",
        ]))
        .unwrap();
    assert!(output.status.success(), "{}", stdout(&output));
    assert!(!stdout(&output).contains("\nside_effect_density "));
}

#[test]
fn selected_density_violation_replays_without_source_or_tools_and_can_change_selection() {
    let directory = workspace();
    select(directory.path(), &["src/bad.rs"]);
    let (output, original) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
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
        assert_eq!(report["mutation_effects"], original["mutation_effects"]);
    }
    select(directory.path(), &["src/clean.rs"]);
    let (output, changed) = replay(
        directory.path(),
        &original,
        &["--changed-files", "selection.json", "--fail-on-findings"],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(scope(&changed)["visible_findings"], 0);
    assert_eq!(
        density(&changed, "bad::bad"),
        density(&original, "bad::bad")
    );
    let (output, unselected) = replay(directory.path(), &original, &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        unselected["mutation_effects"]
            .get("changed_scope")
            .is_none()
    );
    assert!(
        function(&unselected, "bad::bad")
            .get("changed_file")
            .is_none()
    );
}

#[test]
fn density_empty_unknown_and_budgeted_selections_preserve_global_incompleteness() {
    let directory = workspace();
    let (output, original) = requested(directory.path(), &[]);
    assert!(output.status.success());
    let flags = ["--changed-files", "selection.json", "--fail-on-findings"];
    select(directory.path(), &[]);
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
    assert_eq!(limited["mutation_effects"]["execution"], "budget_exhausted");
    select(directory.path(), &["src/missing.rs"]);
    let (output, unknown) = replay(directory.path(), &original, &flags);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(scope(&unknown)["coverage"], "incomplete");
    let selected = selected_path(&unknown, "src/missing.rs");
    assert_eq!(selected["status"], "unobserved");
    assert_eq!(selected["matched_functions"], 0);
    assert_eq!(selected["coverage"], "incomplete");
}

#[test]
fn density_changed_scope_preserves_source_suppression_and_exclusion() {
    let directory = workspace();
    std::fs::write(
        directory.path().join("src/clean.rs"),
        "// rstyle-allow(side_effect_density): reviewed update\npub fn clean(target:&mut u32){*target=1;}\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("effects.toml"),
        format!("exclude=['src/bad.rs']\n{POLICY}"),
    )
    .unwrap();
    select(directory.path(), &["src/clean.rs", "src/bad.rs"]);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(scope(&report)["coverage"], "complete");
    assert_eq!(scope(&report)["visible_findings"], 0);
    assert_eq!(
        density(&report, "clean::clean")["source_suppression"]["status"],
        "allowed"
    );
    assert_eq!(
        density(&report, "bad::bad")["source_exclusion"]["status"],
        "excluded"
    );
    for path in ["src/clean.rs", "src/bad.rs"] {
        assert_eq!(selected_path(&report, path)["matched_functions"], 1);
        assert_eq!(selected_path(&report, path)["visible_findings"], 0);
    }
}

#[test]
fn density_changed_scope_counts_after_saved_review_visibility() {
    let (directory, original) = review_workspace();
    let key = function(&original, "bad")["key"].as_str().unwrap();
    let (output, hidden) = replay(
        directory.path(),
        &original,
        &[
            "--changed-files",
            "selection.json",
            "--review-file",
            "reviews.json",
            "--dismiss-density",
            key,
            "--review-reason",
            "Keep the explicit write.",
            "--fail-on-findings",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(hidden["density_review"]["hidden_findings"], 1);
    assert_eq!(hidden["mutation_effects"]["findings"], 1);
    assert_eq!(hidden["mutation_effects"]["visible_findings"], 0);
    assert_eq!(scope(&hidden)["visible_findings"], 0);
    assert_eq!(selected_path(&hidden, "src/lib.rs")["visible_findings"], 0);
    measured(&hidden, "bad", 1000, "finding");
    select(directory.path(), &["src/clean.rs"]);
    let (_, unselected) = replay(
        directory.path(),
        &original,
        &[
            "--changed-files",
            "selection.json",
            "--review-file",
            "reviews.json",
            "--review-prune",
        ],
    );
    assert_eq!(unselected["density_review"]["hidden_findings"], 0);
    assert_eq!(unselected["review_prune"]["eligible_entries"], 0);
    assert_eq!(scope(&unselected)["visible_findings"], 0);
}

fn review_workspace() -> (TempDir, Value) {
    let directory = workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub mod clean; pub fn bad(target:&mut u32){*target=1;}\n",
    )
    .unwrap();
    select(directory.path(), &["src/lib.rs"]);
    let (_, original) = requested(directory.path(), &["--review-file", "reviews.json"]);
    (directory, original)
}

#[test]
fn unselected_unknown_density_remains_globally_incomplete_for_clean_and_empty_selection() {
    let directory = workspace();
    std::fs::write(
        directory.path().join("src/bad.rs"),
        "pub fn bad(){std::thread::yield_now();}\n",
    )
    .unwrap();
    let (output, original) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(scope(&original)["coverage"], "complete");
    assert_eq!(scope(&original)["visible_findings"], 0);
    assert_eq!(density(&original, "bad::bad")["complete"], false);
    select(directory.path(), &[]);
    let (output, empty) = replay(
        directory.path(),
        &original,
        &["--changed-files", "selection.json", "--fail-on-findings"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(scope(&empty)["coverage"], "complete");
    assert_eq!(scope(&empty)["visible_findings"], 0);
    assert_eq!(empty["mutation_effects"]["coverage"], "incomplete");
}

#[test]
fn source_and_density_changed_scopes_keep_independent_policy_findings() {
    let directory = workspace();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "exclude=['src/bad.rs']\n[fn_params]\nmax_params=0\n",
    )
    .unwrap();
    let (output, original) = requested(
        directory.path(),
        &[
            "--source-rules",
            "--analysis-config",
            "analysis.toml",
            "--fail-on-findings",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(original["analysis"]["changed_scope"]["visible_findings"], 1);
    assert_eq!(scope(&original)["visible_findings"], 0);
    select(directory.path(), &["src/bad.rs"]);
    let (output, changed) = replay(
        directory.path(),
        &original,
        &[
            "--source-rules",
            "--analysis-config",
            "analysis.toml",
            "--changed-files",
            "selection.json",
            "--fail-on-findings",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(changed["analysis"]["changed_scope"]["visible_findings"], 0);
    assert_eq!(scope(&changed)["visible_findings"], 1);
    assert_source_only_replay(directory.path(), &original);
}

fn assert_source_only_replay(directory: &Path, original: &Value) {
    let output = diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap())
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
                "--changed-files",
                "selection.json",
                "--fail-on-findings",
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
            ])
            .env("PATH", directory.join("no-tools")),
    )
    .unwrap();
    assert!(output.status.success(), "{}", stdout(&output));
    let source_only: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        source_only["analysis"]["changed_scope"]["visible_findings"],
        0
    );
    assert_eq!(source_only["mutation_effects"]["density_requested"], false);
    assert!(
        source_only["mutation_effects"]
            .get("changed_scope")
            .is_none()
    );
}

#[test]
fn selected_invalid_density_suppression_remains_a_visible_finding() {
    let directory = workspace();
    std::fs::write(
        directory.path().join("src/clean.rs"),
        "// rstyle-allow(side_effect_density)\npub fn clean(value:u32)->u32{value}\n",
    )
    .unwrap();
    let (output, original) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(original["mutation_effects"]["findings"], 2);
    assert_eq!(scope(&original)["visible_findings"], 1);
    let clean = density(&original, "clean::clean");
    assert_eq!(clean["measurement"]["outcome"], "evaluated_no_finding");
    assert_eq!(
        clean["invalid_suppression"]["rule_id"],
        "invalid_suppression"
    );
    assert_eq!(
        selected_path(&original, "src/clean.rs")["visible_findings"],
        1
    );
    let output =
        diagnostic_process::output(replay_command(directory.path(), &original, "text").args([
            "--changed-files",
            "selection.json",
            "--fail-on-findings",
        ]))
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("invalid_suppression clean::clean"));
    select(directory.path(), &[]);
    let (output, empty) = replay(
        directory.path(),
        &original,
        &["--changed-files", "selection.json", "--fail-on-findings"],
    );
    assert!(output.status.success());
    assert_eq!(scope(&empty)["visible_findings"], 0);
    assert_eq!(empty["mutation_effects"]["findings"], 2);
}
