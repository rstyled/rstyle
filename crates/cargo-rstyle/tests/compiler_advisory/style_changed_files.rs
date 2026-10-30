use super::propagation::{checked, workspace};
use super::*;

const WIDE: &str = "pub fn wide(a:u32,b:u32)->u32 {a+b}\n";

fn fixture() -> TempDir {
    let directory = workspace(WIDE);
    std::fs::write(directory.path().join("caller/src/lib.rs"), WIDE).unwrap();
    std::fs::write(
        directory.path().join("rstyle.toml"),
        "[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    directory
}

fn capture(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let flags = [&["--format", "json"][..], extra].concat();
    let output = rstyle(directory, &flags);
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)));
    (output, report)
}

fn selection(directory: &Path, paths: &[&str]) {
    std::fs::write(
        directory.join("changes.json"),
        serde_json::to_vec(&serde_json::json!({
            "schema":"rstyle.changed-files/1", "paths":paths
        }))
        .unwrap(),
    )
    .unwrap();
}

fn replay(directory: &Path, original: &Value, format: &str) -> Output {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args(["--format", format])
        .args(["--replay-run", original["run_directory"].as_str().unwrap()])
        .args(["--changed-files", "changes.json"])
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    diagnostic_process::output(&mut command).unwrap()
}

fn unchanged(original: &Value, selected: &Value) {
    for field in ["functions", "function_units", "fix_groups"] {
        assert_eq!(original[field], selected[field], "{field}");
    }
    let before = original["compiler_advisory"]["units"].as_array().unwrap();
    let after = selected["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(after) {
        for field in ["unit", "evaluation", "diagnostic_scope", "observations"] {
            assert_eq!(a[field], b[field], "{field}");
        }
    }
}

#[test]
fn root_changed_style_selects_cross_package_findings_and_replays_without_sources_or_tools() {
    let directory = fixture();
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 1);
    assert_eq!(original["diagnostics"].as_array().unwrap().len(), 2);
    selection(directory.path(), &["leaf/src/lib.rs"]);
    let (output, selected) = capture(directory.path(), &["--changed-files", "changes.json"]);
    checked(&output, 1);
    unchanged(&original, &selected);
    assert_eq!(selected["summary_complete"], true);
    assert_eq!(selected["summary"]["functions"], 2);
    assert_eq!(selected["summary"]["violations"], 1);
    assert_eq!(selected["diagnostics"].as_array().unwrap().len(), 1);
    assert_eq!(selected["diagnostics"][0]["path"], "leaf/src/lib.rs");
    assert_eq!(selected["diagnostics"][0]["rule_id"], "fn_params");
    let changed = &selected["compiler_advisory"]["changed_scope"];
    assert_eq!(changed["coverage"], "complete");
    assert_eq!(changed["visible_findings"], 1);
    assert_eq!(changed["paths"][0]["matched_functions"], 1);
    for package in ["leaf", "caller"] {
        std::fs::remove_dir_all(directory.path().join(package).join("src")).unwrap();
        std::fs::remove_file(directory.path().join(package).join("Cargo.toml")).unwrap();
    }
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let output = replay(directory.path(), &original, "json");
    checked(&output, 1);
    let saved: Value = serde_json::from_slice(&output.stdout).unwrap();
    unchanged(&selected, &saved);
    assert_eq!(saved["diagnostics"], selected["diagnostics"]);
    assert_eq!(saved["diagnostic_units"], selected["diagnostic_units"]);
    assert_eq!(saved["compiler_advisory"]["changed_scope"], *changed);
    let output = replay(directory.path(), &original, "text");
    checked(&output, 1);
    let text = stdout(&output);
    assert!(text.contains("Changed-file compiler diagnostics: Complete coverage"));
    assert!(text.contains("leaf/src/lib.rs: Observed, Complete coverage"));
}

#[test]
fn root_changed_style_empty_and_unobserved_paths_keep_distinct_completion() {
    let directory = fixture();
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 1);
    for (paths, code, complete) in [(vec![], 0, true), (vec!["deleted.rs"], 2, false)] {
        selection(directory.path(), &paths);
        let output = replay(directory.path(), &original, "json");
        checked(&output, code);
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        unchanged(&original, &report);
        assert_eq!(report["diagnostics"], serde_json::json!([]));
        assert_eq!(report["summary"]["violations"], 0);
        assert_eq!(report["summary_complete"], complete);
        assert_eq!(report["assessment_complete"], true);
        assert_eq!(report["capture_complete"], true);
        assert_eq!(
            report["compiler_advisory"]["changed_scope"]["coverage"],
            if complete { "complete" } else { "incomplete" }
        );
    }
    for contents in [
        "not JSON",
        r#"{"schema":"rstyle.changed-files/1","paths":["../outside.rs"]}"#,
    ] {
        std::fs::write(directory.path().join("changes.json"), contents).unwrap();
        let output = replay(directory.path(), &original, "json");
        checked(&output, 2);
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("changes.json"));
    }
}

#[test]
fn root_changed_style_does_not_hide_invalid_suppressions_outside_selected_files() {
    let directory = fixture();
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "// rstyle-allow(unknown): invalid control\npub fn narrow() {}\n",
    )
    .unwrap();
    selection(directory.path(), &[]);
    let (output, report) = capture(directory.path(), &["--changed-files", "changes.json"]);
    checked(&output, 1);
    let diagnostics = report["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["rule_id"], "invalid_suppression");
    assert_eq!(diagnostics[0]["path"], "caller/src/lib.rs");
    assert_eq!(report["summary_complete"], true);
    assert_eq!(report["summary"]["violations"], 1);
}

#[test]
fn root_changed_style_empty_selection_does_not_clear_missing_effect_inputs() {
    let directory = workspace("pub fn leaf() {let _=std::fs::read(\"/unused\");}\n");
    selection(directory.path(), &[]);
    let (output, report) = capture(
        directory.path(),
        &["-p", "caller", "--changed-files", "changes.json"],
    );
    checked(&output, 2);
    assert_eq!(report["diagnostics"], serde_json::json!([]));
    assert_eq!(report["summary_complete"], false);
    assert_eq!(report["compiler_advisory"]["complete"], false);
    assert_eq!(report["planning_input_complete"], false);
    let unit = &report["compiler_advisory"]["units"][0];
    assert_eq!(
        unit["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
}

#[test]
fn root_changed_style_reuses_normalized_path_module_locations_in_source_free_replay() {
    let directory = fixture();
    std::fs::create_dir_all(directory.path().join("caller/src/nested")).unwrap();
    std::fs::write(directory.path().join("caller/src/shared.rs"), WIDE).unwrap();
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "#[path=\"nested/../shared.rs\"] pub mod rerouted;\n",
    )
    .unwrap();
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 1);
    selection(directory.path(), &["caller/src/shared.rs"]);
    std::fs::remove_dir_all(directory.path().join("caller/src")).unwrap();
    let output = replay(directory.path(), &original, "json");
    checked(&output, 1);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    unchanged(&original, &report);
    assert_eq!(report["diagnostics"].as_array().unwrap().len(), 1);
    assert_eq!(report["diagnostics"][0]["path"], "caller/src/shared.rs");
    assert_eq!(report["summary_complete"], true);
    let scope = &report["compiler_advisory"]["changed_scope"];
    assert_eq!(scope["coverage"], "complete");
    assert_eq!(scope["unclassified_functions"], 0);
    assert_eq!(scope["paths"][0]["matched_functions"], 1);
    assert_eq!(scope["paths"][0]["visible_findings"], 1);
}
