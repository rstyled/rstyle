use super::*;

const DERIVED: &str = "#[derive(Debug)] pub struct Generated(pub u32);\n";
const RAW: &[&str] = &["--compiler-advisory-only"];

fn capture(source: &str, extra: &[&str]) -> (TempDir, Value) {
    let directory = workspace(source);
    let mut flags = RAW.to_vec();
    flags.extend_from_slice(extra);
    let (output, report) = requested(directory.path(), &flags);
    checked(&output, 0);
    (directory, report)
}

fn source_only(report: &Value) {
    assert_eq!(report["matching_execution"], "not_requested");
    assert_eq!(report["strict_assessment_execution"], "not_requested");
    assert!(report.get("compiler_advisory").is_none());
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
}

fn outside(report: &Value) -> Vec<&Value> {
    report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|function| function["outside_source_rules"] == true)
        .collect()
}

fn assert_generated(report: &Value) {
    let functions = outside(report);
    assert!(!functions.is_empty());
    assert_eq!(
        report["analysis"]["scope"]["outside_source_functions"],
        functions.len()
    );
    for function in functions {
        assert_eq!(function["execution"], "completed");
        for assessment in function["assessments"].as_array().unwrap() {
            assert_eq!(assessment["availability"], "not_applicable");
            assert_eq!(assessment["outcome"], "not_evaluated");
            assert_eq!(assessment["coverage"], "complete");
            assert!(assessment["actual"].is_null());
        }
    }
}

#[test]
fn generated_source_rules_are_outside_applicability_without_hiding_authored_findings() {
    let (directory, original) = capture(
        &format!("{DERIVED}{ORIGINAL}"),
        &["--write-source-baseline", "baseline.json"],
    );
    source_only(&original);
    assert_generated(&original);
    assert_eq!(original["analysis"]["findings"], 3);
    assert_eq!(
        stored(directory.path())["entries"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let output = replay(directory.path(), &original, &[RAW, BASELINE].concat());
    checked(&output, 0);
    let report = parsed(&output);
    source_only(&report);
    counts(&report, 3, 0);
    let output = replay(
        directory.path(),
        &original,
        &["--compiler-advisory-only", "--fail-on-findings"],
    );
    checked(&output, 1);
    let output = replay(
        directory.path(),
        &original,
        &["--compiler-advisory-only", "--compiler-advisory"],
    );
    checked(&output, 2);
    let combined = parsed(&output);
    assert_eq!(combined["compiler_advisory"]["complete"], false);
    assert_eq!(combined["analysis"], original["analysis"]);
}

#[test]
fn generated_source_applicability_and_empty_baseline_replay_without_sources_or_tools() {
    let (directory, original) = capture(
        &format!("{DERIVED}pub fn clean(value:u32)->u32 {{value}}\n"),
        &["--write-source-baseline", "baseline.json"],
    );
    source_only(&original);
    assert_generated(&original);
    assert_eq!(original["analysis"]["findings"], 0);
    assert!(
        stored(directory.path())["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let mut flags = [RAW, BASELINE].concat();
        flags.extend(["--report-detail", detail]);
        let output = replay(directory.path(), &original, &flags);
        checked(&output, 0);
        let report = parsed(&output);
        source_only(&report);
        counts(&report, 0, 0);
        assert_eq!(report["analysis"], original["analysis"]);
        assert_eq!(report["verification_policy"], "raw-replay");
    }
}

#[test]
fn generated_only_changed_files_are_known_outside_source_rules_but_unknown_paths_are_incomplete() {
    let directory = workspace("pub mod generated; pub fn clean() {}\n");
    std::fs::write(directory.path().join("src/generated.rs"), DERIVED).unwrap();
    let (output, original) = requested(directory.path(), RAW);
    checked(&output, 0);
    for (path, code) in [("src/generated.rs", 0), ("src/missing.rs", 2)] {
        select(directory.path(), &[path]);
        let output = replay(
            directory.path(),
            &original,
            &[
                "--compiler-advisory-only",
                "--changed-files",
                "selection.json",
                "--fail-on-findings",
            ],
        );
        checked(&output, code);
        let report = parsed(&output);
        let selected = &report["analysis"]["changed_scope"]["paths"][0];
        assert_eq!(
            selected["coverage"],
            if code == 0 { "complete" } else { "incomplete" }
        );
        if code == 0 {
            assert_eq!(
                selected["outside_scope_functions"],
                outside(&original).len()
            );
        }
        assert_eq!(report["analysis"]["findings"], 0);
    }
    let output = replay(
        directory.path(),
        &original,
        &["--compiler-advisory-only", "--guard-clauses"],
    );
    checked(&output, 2);
    let report = parsed(&output);
    assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
    assert!(outside(&report).iter().all(|function| {
        function["guard_clause"]["outcome"] == "not_evaluated"
            && function["guard_clause"]["coverage"] == "incomplete"
    }));
}

fn raw_wire(report: &Value) -> Value {
    let bundle = report["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    serde_json::from_slice(&std::fs::read(bundle).unwrap()).unwrap()
}

fn damage(wire: &Value, kind: &str) -> Value {
    let mut changed = wire.clone();
    for function in changed["facts"].as_array_mut().unwrap() {
        if function["name"] == "clean" && kind == "missing_authored_owner" {
            function["build"]["frontend_options"]
                .as_object_mut()
                .unwrap()
                .remove(rstyle_pdg::nominal_boundary::CONTEXT_KEY);
        } else if function["name"] != "clean" {
            if kind == "unknown_origin" {
                function["source_diagnostics"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|diagnostic| diagnostic["construct"] != "macro_generated_function");
            } else if kind == "invalid_generated_bytes" {
                function["source"]["digest"] = "xxh3-64:0000000000000000".into();
            }
        }
    }
    changed
}

#[test]
fn source_applicability_does_not_mask_missing_authored_ownership_unknown_origin_or_invalid_bytes() {
    let (directory, original) = capture(&format!("{DERIVED}pub fn clean() {{}}\n"), &[]);
    let wire = raw_wire(&original);
    let ledger = super::super::migration_compiler_advisory::only::ledger_path(&original);
    for kind in [
        "missing_authored_owner",
        "unknown_origin",
        "invalid_generated_bytes",
    ] {
        let changed = damage(&wire, kind);
        super::super::migration_compiler_advisory::only::reseal_raw(&original, &ledger, &changed);
        let output = replay(
            directory.path(),
            &original,
            &[
                "--compiler-advisory-only",
                "--write-source-baseline",
                "invalid.json",
            ],
        );
        checked(&output, 2);
        assert!(!directory.path().join("invalid.json").exists(), "{kind}");
        if let Ok(report) = serde_json::from_slice::<Value>(&output.stdout) {
            assert_ne!(
                report["analysis"]["scope"]["coverage"], "complete",
                "{kind}: {report}"
            );
        }
    }
    super::super::migration_compiler_advisory::only::reseal_raw(&original, &ledger, &wire);
    checked(&replay(directory.path(), &original, RAW), 0);
}

fn authored<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == name)
        .unwrap()
}

fn dismiss_authored_parameter(directory: &Path, original: &Value) {
    let output = replay(
        directory,
        original,
        &[
            "--compiler-advisory-only",
            "--review-file",
            "reviews.json",
            "--dismiss-parameter",
            authored(original, "many")["key"].as_str().unwrap(),
            "--review-reason",
            "Reviewed authored boundary",
        ],
    );
    checked(&output, 0);
}

#[test]
fn generated_replacements_never_resolve_or_prune_a_reviewed_authored_boundary() {
    let (directory, original) = capture(ORIGINAL, &["--write-source-baseline", "baseline.json"]);
    dismiss_authored_parameter(directory.path(), &original);
    let saved = std::fs::read(directory.path().join("reviews.json")).unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{DERIVED}{ORIGINAL}"),
    )
    .unwrap();
    let (output, with_derive) = requested(
        directory.path(),
        &["--compiler-advisory-only", "--review-file", "reviews.json"],
    );
    checked(&output, 0);
    assert_eq!(
        with_derive["parameter_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    let generated = generated_replacement(directory.path());
    let output = replay(
        directory.path(),
        &generated,
        &[
            "--compiler-advisory-only",
            "--review-file",
            "reviews.json",
            "--review-status",
            "--review-prune",
            "--review-prune-dry-run",
        ],
    );
    let report = parsed(&output);
    assert_ne!(report["review_status"]["records"][0]["state"], "no_finding");
    assert_eq!(report["review_prune"]["eligible_entries"], 0);
    assert_eq!(report["review_prune"]["removed_entries"], 0);
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        saved
    );
}

fn generated_replacement(directory: &Path) -> Value {
    std::fs::write(
        directory.join("src/lib.rs"),
        "macro_rules! make {()=>{pub fn many(a:u32,b:u32)->u32{a+b}}} make!();\n",
    )
    .unwrap();
    let (output, generated) = requested(
        directory,
        &[
            "--compiler-advisory-only",
            "--review-file",
            "reviews.json",
            "--review-status",
            "--source-baseline",
            "baseline.json",
            "--fail-on-new-source",
        ],
    );
    assert!(
        matches!(output.status.code(), Some(0 | 2)),
        "{}",
        stdout(&output)
    );
    let retained = generated["source_baseline"]["retained"].as_array().unwrap();
    assert_eq!(retained.len(), 3);
    for entry in retained {
        assert!(
            matches!(entry["state"].as_str(), Some("unavailable" | "unobserved")),
            "{entry}"
        );
    }
    assert_ne!(
        generated["review_status"]["records"][0]["state"],
        "no_finding"
    );
    generated
}
