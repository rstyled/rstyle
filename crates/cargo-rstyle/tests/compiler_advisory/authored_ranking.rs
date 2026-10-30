use super::propagation::checked;
use super::*;

const DERIVED: &str = "#[derive(Debug)] pub struct Generated(pub u32);\n";
const ORIGINAL: &str = "pub fn original(a:bool,b:bool){if a {if b {}}}\n";
const RANK: &[&str] = &[
    "--extraction-profile",
    "hir-data-v3",
    "--compiler-advisory-only",
    "--compiler-complexity-top",
    "10",
];
const AUTHORED: &[&str] = &["--compiler-complexity-scope", "authored"];

fn capture_rank(source: &str, extra: &[&str]) -> (TempDir, Output, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let mut flags = RANK.to_vec();
    flags.extend_from_slice(extra);
    let (output, report) = clones(directory.path(), &flags);
    (directory, output, report)
}

fn replay_rank(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .arg("clones")
        .args(RANK)
        .args([
            "--format",
            "json",
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

fn unit(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]
}

fn ranking(report: &Value) -> &Value {
    &unit(report)["complexity_ranking"]
}

fn outside(ranking: &Value) -> &Vec<Value> {
    assert_eq!(ranking["source_scope"]["kind"], "authored");
    ranking["source_scope"]["outside_scope_functions"]
        .as_array()
        .unwrap()
}

fn standalone(report: &Value) {
    let advisory = &report["compiler_advisory"];
    assert_eq!(advisory["diagnostic_execution"], "not_requested");
    assert_eq!(advisory["effect_execution"], "not_requested");
    assert_eq!(advisory["findings"], 0);
    assert_eq!(report["matching_execution"], "not_requested");
    assert_eq!(report["strict_assessment_execution"], "not_requested");
    for unit in advisory["units"].as_array().unwrap() {
        assert!(unit.get("evaluation").is_none());
        assert!(unit.get("patterns").is_none());
    }
}

#[test]
fn authored_complexity_scope_preserves_captured_default_scores_and_raw_missing_roster() {
    let (directory, output, captured) = capture_rank(&format!("{DERIVED}{ORIGINAL}"), &[]);
    checked(&output, 2);
    standalone(&captured);
    assert!(ranking(&captured).get("source_scope").is_none());
    let (output, explicit) = replay_rank(
        directory.path(),
        &captured,
        &["--compiler-complexity-scope", "captured"],
    );
    checked(&output, 2);
    assert_eq!(explicit["compiler_advisory"], captured["compiler_advisory"]);
    let (output, authored) = replay_rank(directory.path(), &captured, AUTHORED);
    checked(&output, 0);
    standalone(&authored);
    assert_eq!(authored["compiler_advisory"]["complete"], true);
    assert_eq!(ranking(&authored)["input_complete"], true);
    assert_eq!(ranking(&authored)["unavailable_owners"], 0);
    same_inputs(&captured, &authored);
    numeric_unchanged(directory.path(), &captured, &authored);
    std::fs::write(
        directory.path().join("excluded.toml"),
        "exclude=['src/**']\n",
    )
    .unwrap();
    let (output, excluded) = replay_rank(
        directory.path(),
        &captured,
        &[AUTHORED, &["--compiler-advisory-config", "excluded.toml"]].concat(),
    );
    checked(&output, 0);
    assert_eq!(ranking(&excluded)["known_functions"], 0);
    assert_eq!(ranking(&excluded)["functions"], serde_json::json!([]));
    assert_eq!(
        unit(&excluded)["unavailable"],
        unit(&captured)["unavailable"]
    );
}

fn numeric_unchanged(directory: &Path, captured: &Value, authored: &Value) {
    let (output, before) = replay_rank(directory, captured, &["--compiler-advisory"]);
    checked(&output, 2);
    let (output, after) = replay_rank(
        directory,
        captured,
        &[AUTHORED, &["--compiler-advisory"]].concat(),
    );
    checked(&output, 2);
    assert_eq!(after["compiler_advisory"]["complete"], false);
    assert_eq!(ranking(&after), ranking(authored));
    for field in [
        "evaluation",
        "observations",
        "unavailable",
        "unresolved_callees",
    ] {
        assert_eq!(unit(&before)[field], unit(&after)[field], "{field}");
    }
    assert_eq!(
        before["compiler_advisory"]["findings"],
        after["compiler_advisory"]["findings"]
    );
}

fn same_inputs(captured: &Value, authored: &Value) {
    let missing: Vec<_> = unit(captured)["unavailable"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["key"].clone())
        .collect();
    assert!(!missing.is_empty());
    assert_eq!(outside(ranking(authored)), &missing);
    for field in ["observations", "unavailable", "unavailable_complexity"] {
        assert_eq!(unit(authored)[field], unit(captured)[field], "{field}");
    }
    for field in [
        "functions",
        "owners",
        "known_functions",
        "measured_functions",
    ] {
        assert_eq!(
            ranking(authored)[field],
            ranking(captured)[field],
            "{field}"
        );
    }
}

#[test]
fn authored_generated_only_complexity_ranking_is_complete_without_inventing_scores() {
    let (directory, output, original) = capture_rank(DERIVED, AUTHORED);
    checked(&output, 0);
    standalone(&original);
    let result = ranking(&original);
    assert_eq!(result["input_complete"], true);
    assert_eq!(result["known_functions"], 0);
    assert_eq!(result["measured_functions"], 0);
    assert_eq!(result["functions"], serde_json::json!([]));
    assert!(!outside(result).is_empty());
    assert!(
        !unit(&original)["unavailable"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, replayed) = replay_rank(directory.path(), &original, AUTHORED);
    checked(&output, 0);
    assert_eq!(replayed["compiler_advisory"], original["compiler_advisory"]);
}

fn raw_wire(report: &Value) -> Value {
    let path = report["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn damaged(wire: &Value, kind: &str) -> Value {
    let mut changed = wire.clone();
    for fact in changed["facts"].as_array_mut().unwrap() {
        if fact["name"] == "original" {
            let options = fact["build"]["frontend_options"].as_object_mut().unwrap();
            match kind {
                "score" => options[rstyle_core::advisory::KEY]["facts"]["complexity"] = Value::Null,
                "observation" => {
                    options.remove(rstyle_core::advisory::KEY);
                }
                _ => {}
            }
        } else if kind == "origin" {
            fact["source_diagnostics"]
                .as_array_mut()
                .unwrap()
                .retain(|row| row["construct"] != "macro_generated_function");
            let options = fact["build"]["frontend_options"].as_object_mut().unwrap();
            options.remove(rstyle_core::advisory::generated_effects::KEY);
            options.remove(rstyle_core::advisory::generated_effects::CONNECTED_KEY);
        }
    }
    changed
}

#[test]
fn authored_scope_does_not_hide_missing_authored_scores_observations_or_unknown_origins() {
    let (directory, output, original) = capture_rank(&format!("{DERIVED}{ORIGINAL}"), AUTHORED);
    checked(&output, 0);
    let wire = raw_wire(&original);
    let ledger = only::ledger_path(&original);
    for kind in ["score", "observation", "origin"] {
        only::reseal_raw(&original, &ledger, &damaged(&wire, kind));
        let (output, report) = replay_rank(directory.path(), &original, AUTHORED);
        checked(&output, 2);
        standalone(&report);
        assert_eq!(ranking(&report)["input_complete"], false, "{kind}");
        if kind == "score" {
            assert_eq!(ranking(&report)["unavailable_measurements"], 1);
        } else if kind == "origin" {
            assert!(outside(ranking(&report)).is_empty());
            assert_eq!(ranking(&report)["unavailable_owners"], 1);
        } else {
            assert_eq!(ranking(&report)["unavailable_owners"], 1);
        }
    }
    only::reseal_raw(&original, &ledger, &wire);
    checked(&replay_rank(directory.path(), &original, AUTHORED).0, 0);
}

#[test]
fn authored_ranking_retains_check_and_test_owners_and_replays_without_sources_or_tools() {
    let source = format!("{DERIVED}{ORIGINAL}#[cfg(test)] pub fn test_only(a:bool){{if a {{}}}}\n");
    let (directory, output, original) =
        capture_rank(&source, &[AUTHORED, &["--all-targets"]].concat());
    checked(&output, 0);
    standalone(&original);
    let units = original["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    assert_ne!(units[0]["unit"], units[1]["unit"]);
    let mut measured: Vec<_> = units
        .iter()
        .map(|unit| {
            let ranking = &unit["complexity_ranking"];
            assert_eq!(ranking["input_complete"], true);
            assert!(!outside(ranking).is_empty());
            ranking["measured_functions"].as_u64().unwrap()
        })
        .collect();
    measured.sort_unstable();
    assert_eq!(measured, [1, 2]);
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, report) = replay_rank(directory.path(), &original, AUTHORED);
    checked(&output, 0);
    assert_eq!(report["compiler_advisory"], original["compiler_advisory"]);
}

#[test]
fn complexity_scope_requires_ranking_before_capture_even_when_explicitly_captured() {
    let directory = clone_workspace();
    for scope in ["captured", "authored"] {
        let output = diagnostic_process::output(
            Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap())
                .current_dir(directory.path())
                .args(["clones", "--compiler-complexity-scope", scope])
                .env("PATH", directory.path().join("no-tools"))
                .env("CARGO", directory.path().join("no-cargo")),
        )
        .unwrap();
        checked(&output, 2);
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("--compiler-complexity-top"), "{error}");
        assert!(!error.contains("no-cargo"), "{error}");
        assert!(!directory.path().join("target/rstyle/clones").exists());
    }
}
