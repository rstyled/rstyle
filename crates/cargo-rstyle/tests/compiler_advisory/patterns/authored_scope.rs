use super::mining_options::{checked, remove_sources};
use super::*;

const DERIVED: &str = "#[derive(Debug)] pub struct Generated(pub u32);";
const HELPERS: &str = "pub fn first(a:u128)->u128 {std::hint::black_box(a.wrapping_add(7)).wrapping_mul(3)} pub fn second(a:u128)->u128 {std::hint::black_box(a.wrapping_add(8)).wrapping_mul(3)}";
const AUTHORED: &[&str] = &["--compiler-pattern-scope", "authored"];

fn fixture(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    directory
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: {}\n{}",
            stdout(output),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn raw(directory: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["patterns", "--format", "json"];
    args.extend_from_slice(extra);
    rstyle(directory, &args)
}

fn saved(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "patterns",
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
    let value = json(&output);
    (output, value)
}

fn direct(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-patterns",
        "--pattern-ledger",
        "rstyle-ignore.toml",
        "--compiler-advisory-only",
        "--report-detail",
        "compact",
        "--replay-run",
        original["run_directory"].as_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn outside(report: &Value) -> &Vec<Value> {
    proposals(report)["source_scope"]["outside_scope_functions"]
        .as_array()
        .unwrap()
}

#[test]
fn authored_scope_distinguishes_derived_owners_from_explicit_captured_scope() {
    let directory = fixture(&format!("{DERIVED} pub fn original() {{}}"));
    let output = raw(directory.path(), &["--compiler-pattern-scope", "captured"]);
    checked(&output, 2);
    let original = json(&output);
    assert!(proposals(&original).get("source_scope").is_none());
    assert_eq!(proposals(&original)["input_complete"], false);
    let (output, authored) = saved(directory.path(), &original, &[]);
    checked(&output, 0);
    assert_eq!(proposals(&authored)["source_scope"]["kind"], "authored");
    assert_eq!(proposals(&authored)["input_complete"], true);
    assert_eq!(proposals(&authored)["unavailable_functions"], 0);
    assert!(!outside(&authored).is_empty());
    let unit = &authored["compiler_advisory"]["units"][0];
    let excluded: Vec<_> = unit["unavailable"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["key"].clone())
        .collect();
    assert_eq!(outside(&authored), &excluded);
    assert_eq!(
        unit["unavailable"],
        original["compiler_advisory"]["units"][0]["unavailable"]
    );
    assert_eq!(
        proposals(&authored)["evaluation"],
        proposals(&original)["evaluation"]
    );
    let (output, captured) = saved(
        directory.path(),
        &original,
        &["--compiler-pattern-scope", "captured"],
    );
    checked(&output, 2);
    assert_eq!(captured["compiler_advisory"], original["compiler_advisory"]);
    let (output, explicit) = direct(directory.path(), &original, AUTHORED);
    checked(&output, 0);
    assert_eq!(explicit["compiler_advisory"], authored["compiler_advisory"]);
}

#[test]
fn authored_scope_preserves_findings_and_replays_without_sources_or_tools() {
    let directory = fixture(&format!("{DERIVED} {HELPERS}"));
    let output = raw(
        directory.path(),
        &[
            "--compiler-pattern-scope",
            "authored",
            "--fail-on-candidates",
        ],
    );
    checked(&output, 1);
    let original = json(&output);
    assert_eq!(proposals(&original)["input_complete"], true);
    assert_eq!(
        proposals(&original)["evaluation"]["candidates"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    remove_sources(directory.path());
    let (output, replayed) = saved(
        directory.path(),
        &original,
        &[
            "--compiler-pattern-scope",
            "authored",
            "--fail-on-candidates",
        ],
    );
    checked(&output, 1);
    assert_eq!(replayed["compiler_advisory"], original["compiler_advisory"]);
    assert_eq!(replayed["matching_execution"], "not_requested");
    assert_eq!(replayed["strict_assessment_execution"], "not_requested");
}

#[test]
fn authored_scope_preserves_numeric_incompleteness_and_strict_decisions() {
    let directory = fixture(&format!("{DERIVED} pub fn original() {{}}"));
    let output = raw(directory.path(), AUTHORED);
    checked(&output, 0);
    let original = json(&output);
    let (output, combined) = direct(
        directory.path(),
        &original,
        &[
            "--compiler-pattern-scope",
            "authored",
            "--compiler-advisory",
        ],
    );
    checked(&output, 2);
    assert_eq!(proposals(&combined)["input_complete"], true);
    assert_eq!(combined["compiler_advisory"]["complete"], false);
    strict_decisions_agree(directory.path());
}

fn strict_decisions_agree(directory: &Path) {
    let (output, standalone) = clones(directory, &["--extraction-profile", "hir-data-v3"]);
    checked(&output, 0);
    let (output, strict) = clones(
        directory,
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-patterns",
            "--compiler-pattern-scope",
            "authored",
        ],
    );
    checked(&output, 0);
    assert_eq!(strict["matching_execution"], "completed");
    assert_eq!(strict["matching"], standalone["matching"]);
    assert_eq!(strict["accepted_pairs"], standalone["accepted_pairs"]);
    let outcomes = |report: &Value| {
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|function| (function["name"].clone(), function["outcome"].clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(outcomes(&strict), outcomes(&standalone));
    assert!(
        !strict["compiler_advisory"]["units"][0]["unavailable"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn authored_scope_keeps_check_and_test_owner_populations_distinct() {
    let directory = fixture(&format!(
        "{DERIVED} pub fn original() {{}} #[cfg(test)] pub fn test_only() {{}}"
    ));
    let output = raw(
        directory.path(),
        &["--compiler-pattern-scope", "authored", "--all-targets"],
    );
    checked(&output, 0);
    let report = json(&output);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    assert_ne!(units[0]["unit"], units[1]["unit"]);
    let mut counts: Vec<_> = units
        .iter()
        .map(|unit| {
            assert_eq!(unit["patterns"]["input_complete"], true);
            assert_eq!(unit["patterns"]["source_scope"]["kind"], "authored");
            assert!(
                !unit["patterns"]["source_scope"]["outside_scope_functions"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            unit["patterns"]["observed_graphs"].as_u64().unwrap()
        })
        .collect();
    counts.sort();
    assert_eq!(counts, [1, 2]);
    let (output, replayed) = saved(directory.path(), &report, AUTHORED);
    checked(&output, 0);
    assert_eq!(replayed["compiler_advisory"], report["compiler_advisory"]);
}

#[test]
fn authored_scope_can_export_a_baseline_despite_expanded_owners() {
    let directory = fixture(&format!("{DERIVED} {HELPERS}"));
    let output = raw(directory.path(), AUTHORED);
    checked(&output, 0);
    let original = json(&output);
    let (output, exported) = direct(
        directory.path(),
        &original,
        &[
            "--compiler-pattern-scope",
            "authored",
            "--write-compiler-pattern-baseline",
            "baseline.json",
        ],
    );
    checked(&output, 0);
    let bytes = std::fs::read(directory.path().join("baseline.json")).unwrap();
    let baseline: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(baseline["source_scope"], "authored");
    assert_eq!(baseline["entries"].as_array().unwrap().len(), 1);
    assert_eq!(
        proposals(&exported)["evaluation"],
        proposals(&original)["evaluation"]
    );
    remove_sources(directory.path());
    let (output, compared) = direct(
        directory.path(),
        &original,
        &[
            "--compiler-pattern-scope",
            "authored",
            "--compiler-pattern-baseline",
            "baseline.json",
            "--fail-on-new-compiler-patterns",
        ],
    );
    checked(&output, 0);
    assert_eq!(
        compared["compiler_advisory"]["pattern_baseline"]["scope_changed"],
        false
    );
    assert_eq!(
        compared["compiler_advisory"]["pattern_baseline"]["existing_findings"],
        1
    );
    assert_eq!(
        std::fs::read(directory.path().join("baseline.json")).unwrap(),
        bytes
    );
}

#[test]
fn source_scope_does_not_change_a_positive_structural_review() {
    let directory = fixture(&format!("{DERIVED} {HELPERS}"));
    let output = raw(directory.path(), AUTHORED);
    checked(&output, 0);
    let original = json(&output);
    let id = proposals(&original)["evaluation"]["candidates"][0]["id"]
        .as_str()
        .unwrap();
    let (output, reviewed) = direct(
        directory.path(),
        &original,
        &[
            "--compiler-pattern-scope",
            "authored",
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-pattern",
            "0",
            id,
            "--review-reason",
            "Independent authored-scope fixture review",
        ],
    );
    checked(&output, 0);
    let decisions = &reviewed["compiler_advisory"]["pattern_review"]["decisions"];
    assert_eq!(decisions[0]["state"], "hidden_exact");
    let bytes = std::fs::read(directory.path().join("reviews.json")).unwrap();
    remove_sources(directory.path());
    let (output, captured) = saved(
        directory.path(),
        &original,
        &[
            "--review-file",
            "reviews.json",
            "--review-status",
            "--compiler-pattern-scope",
            "captured",
        ],
    );
    checked(&output, 2);
    assert_eq!(
        &captured["compiler_advisory"]["pattern_review"]["decisions"],
        decisions
    );
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        bytes
    );
}

#[test]
fn authored_and_captured_baseline_headers_cannot_establish_each_others_clean_scope() {
    let directory = fixture("pub fn original() {}");
    let output = raw(directory.path(), &[]);
    checked(&output, 0);
    let original = json(&output);
    for (scope, filename) in [("captured", "captured.json"), ("authored", "authored.json")] {
        let (output, _) = direct(
            directory.path(),
            &original,
            &[
                "--compiler-pattern-scope",
                scope,
                "--write-compiler-pattern-baseline",
                filename,
            ],
        );
        checked(&output, 0);
        let baseline: Value =
            serde_json::from_slice(&std::fs::read(directory.path().join(filename)).unwrap())
                .unwrap();
        if scope == "captured" {
            assert!(baseline.get("source_scope").is_none());
        } else {
            assert_eq!(baseline["source_scope"], "authored");
        }
    }
    for (scope, filename) in [("authored", "captured.json"), ("captured", "authored.json")] {
        let (output, report) = direct(
            directory.path(),
            &original,
            &[
                "--compiler-pattern-scope",
                scope,
                "--compiler-pattern-baseline",
                filename,
                "--fail-on-new-compiler-patterns",
            ],
        );
        checked(&output, 1);
        assert_eq!(
            report["compiler_advisory"]["pattern_baseline"]["scope_changed"],
            true
        );
    }
}

fn select(directory: &Path, path: &str) {
    std::fs::write(
        directory.join("selection.json"),
        serde_json::to_vec(&serde_json::json!({
            "schema": "rstyle.changed-files/1", "paths": [path],
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn authored_changed_scope_covers_expanded_only_files_but_not_unknown_paths() {
    let directory = fixture("pub mod generated; pub fn original() {}");
    std::fs::write(directory.path().join("src/generated.rs"), DERIVED).unwrap();
    let output = raw(directory.path(), AUTHORED);
    checked(&output, 0);
    let original = json(&output);
    for (path, code) in [
        ("src/lib.rs", 0),
        ("src/generated.rs", 0),
        ("src/missing.rs", 2),
    ] {
        select(directory.path(), path);
        let (output, report) = direct(
            directory.path(),
            &original,
            &[
                "--compiler-pattern-scope",
                "authored",
                "--select-compiler-patterns",
                "--changed-files",
                "selection.json",
                "--fail-on-findings",
            ],
        );
        checked(&output, code);
        assert_eq!(proposals(&report)["input_complete"], true);
        let coverage = &report["compiler_advisory"]["changed_scope"]["paths"][0]["coverage"];
        assert_eq!(coverage, if code == 0 { "complete" } else { "incomplete" });
    }
}

fn raw_wire(original: &Value) -> Value {
    let path = original["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn damaged_wire(wire: &Value, damage: &str) -> Value {
    let mut wire = wire.clone();
    for function in wire["facts"].as_array_mut().unwrap() {
        if function["name"] == "original" {
            let options = function["build"]["frontend_options"]
                .as_object_mut()
                .unwrap();
            match damage {
                "observation" => {
                    options.remove(rstyle_core::advisory::KEY);
                }
                "graph" => {
                    options.remove(rstyle_core::advisory::patterns::KEY);
                }
                "historical" => {
                    options[rstyle_core::advisory::KEY]["metrics"] =
                        rstyle_core::advisory::V6_METRICS.into();
                }
                _ => {}
            }
        } else if matches!(damage, "unknown_origin" | "historical") {
            let options = function["build"]["frontend_options"]
                .as_object_mut()
                .unwrap();
            options.remove(rstyle_core::advisory::generated_effects::KEY);
            options.remove(rstyle_core::advisory::generated_effects::CONNECTED_KEY);
            if damage == "unknown_origin" {
                function["source_diagnostics"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|row| row["construct"] != "macro_generated_function");
            }
        }
    }
    wire
}

fn missing_graph_selection(directory: &Path, original: &Value, current: &Value) {
    select(directory, "src/lib.rs");
    let (output, selected) = direct(
        directory,
        original,
        &[
            "--compiler-pattern-scope",
            "authored",
            "--select-compiler-patterns",
            "--changed-files",
            "selection.json",
            "--fail-on-findings",
        ],
    );
    checked(&output, 2);
    assert_eq!(proposals(&selected)["input_complete"], false);
    assert_eq!(
        selected["compiler_advisory"]["changed_scope"]["paths"][0]["coverage"],
        "incomplete"
    );
    assert_eq!(
        proposals(&selected)["evaluation"],
        proposals(current)["evaluation"]
    );
}

#[test]
fn authored_scope_never_hides_missing_authored_graphs_observations_or_unknown_origins() {
    let directory = fixture(&format!("{DERIVED} pub fn original() {{}}"));
    let output = raw(directory.path(), AUTHORED);
    checked(&output, 0);
    let original = json(&output);
    let wire = raw_wire(&original);
    let ledger = super::super::only::ledger_path(&original);
    for damage in ["observation", "graph", "unknown_origin", "historical"] {
        let changed = damaged_wire(&wire, damage);
        super::super::only::reseal_raw(&original, &ledger, &changed);
        let (output, report) = saved(directory.path(), &original, AUTHORED);
        let complete = damage == "historical";
        checked(&output, if complete { 0 } else { 2 });
        assert_eq!(proposals(&report)["input_complete"], complete, "{damage}");
        assert_eq!(report["capture_failures"], serde_json::json!([]));
        assert_eq!(report["handoff_failures"], serde_json::json!([]));
        if damage == "unknown_origin" {
            assert!(outside(&report).is_empty());
            assert_eq!(proposals(&report)["unavailable_functions"], 1);
        }
        if damage == "graph" {
            assert_eq!(
                proposals(&report)["unavailable_graphs"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
            missing_graph_selection(directory.path(), &original, &report);
        }
    }
    super::super::only::reseal_raw(&original, &ledger, &wire);
}
