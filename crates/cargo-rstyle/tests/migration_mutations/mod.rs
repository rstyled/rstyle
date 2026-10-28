use super::*;

fn labels(id: &str) -> Value {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../eval/clones-capability/effects-cases.json"
    ))
    .unwrap();
    cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap()
        .clone()
}

fn workspace(id: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        labels(id)["source"].as_str().unwrap(),
    )
    .unwrap();
    policy(directory.path(), 2);
    directory
}

fn policy(directory: &Path, writes: usize) {
    std::fs::write(
        directory.join("mutation.toml"),
        format!("[mutation_budget]\nmax_mutations_per_target={writes}\nmax_mutated_targets=1\n"),
    )
    .unwrap();
}

fn requested(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--mutation-rules",
        "--mutation-config",
        "mutation.toml",
    ];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn function(report: &Value) -> &Value {
    let functions = report["mutation_analysis"]["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 1);
    &functions[0]
}

fn rule<'a>(function: &'a Value, id: &str) -> &'a Value {
    function["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["rule_id"] == id)
        .unwrap()
}

fn assert_bound(actual: &Value, kind: &str, value: u64) {
    assert_eq!(actual["kind"], kind);
    assert_eq!(actual["value"], value);
}

fn assert_span(span: &Value, source: &str) {
    let start = span["start"].as_u64().unwrap() as usize;
    let end = span["end"].as_u64().unwrap() as usize;
    assert!(
        start <= end && end <= source.len(),
        "invalid source span {span}"
    );
    assert!(source.is_char_boundary(start) && source.is_char_boundary(end));
}

#[test]
fn direct_scalar_assignment_counts_follow_independent_sources_and_provenance() {
    for id in [
        "read-only-scalars",
        "local-write-at-limit",
        "owned-formal-rebinding-over-limit",
        "borrowed-formal-pointee-writes",
        "builtin-compound-assignment-once",
        "two-local-targets",
    ] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path(), &[]);
        assert!(output.status.success(), "{id}: {}", stdout(&output));
        assert_exact_case(&report, &labels(id));
    }
}

fn assert_exact_case(report: &Value, case: &Value) {
    let analysis = &report["mutation_analysis"];
    assert_eq!(analysis["schema"], "rstyle.mutations/1");
    assert_eq!(analysis["execution"], "completed");
    assert_eq!(analysis["coverage"], "complete");
    assert_eq!(
        analysis["assessed_functions"],
        analysis["expected_functions"]
    );
    let function = function(report);
    let expected = &case["expected"];
    assert_eq!(function["availability"], "known");
    assert_eq!(function["coverage"], "complete");
    let evidence = &function["evidence"];
    for count in ["write_sites", "mutated_roots", "escaped_write_sites"] {
        assert_eq!(evidence[count], expected[count], "{count}: {case}");
    }
    for id in ["mutation_per_target", "mutated_targets"] {
        assert_eq!(rule(function, id)["outcome"], expected[id]);
    }
    assert_eq!(function["density"]["outcome"], "not_evaluated");
    assert!(!function["density"]["reason"].as_str().unwrap().is_empty());
    let source = case["source"].as_str().unwrap();
    assert_sites(evidence, source);
    if let Some(expected) = expected["root_provenance"].as_array() {
        assert_root_provenance(evidence, expected);
    }
}

fn assert_sites(evidence: &Value, source: &str) {
    let mut escaped = 0;
    for site in evidence["sites"].as_array().unwrap() {
        assert_span(&site["span"], source);
        assert_span(&site["root"]["declaration_span"], source);
        assert_span(&site["root"]["scope_span"], source);
        assert!(!site["root"]["native"].is_null());
        let provenance = site["root"]["provenance"].as_str().unwrap();
        assert!(["local_owned", "formal_owned", "formal_borrowed"].contains(&provenance));
        if provenance != "local_owned" {
            assert!(site["root"]["formal"].as_u64().is_some());
        }
        assert!(
            ["assignment", "compound_assignment"].contains(&site["operation"].as_str().unwrap())
        );
        let visible = site["caller_visible"].as_bool().unwrap();
        assert_eq!(visible, provenance == "formal_borrowed");
        escaped += usize::from(visible);
    }
    assert_eq!(
        evidence["escaped_write_sites"]["value"].as_u64().unwrap() as usize,
        escaped
    );
}

fn assert_root_provenance(evidence: &Value, expected: &[Value]) {
    let mut roots = std::collections::BTreeMap::new();
    for site in evidence["sites"].as_array().unwrap() {
        let native = &site["root"]["native"];
        let key = (
            native["krate"].as_u64().unwrap(),
            native["owner"].as_u64().unwrap(),
            native["local"].as_u64().unwrap(),
        );
        roots.insert(key, site["root"]["provenance"].as_str().unwrap());
    }
    let mut actual: Vec<_> = roots.into_values().collect();
    let mut expected: Vec<_> = expected
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn unknown_calls_keep_bounds_incomplete_without_hiding_witnessed_breaches() {
    for (id, writes, outcome) in [
        ("unknown-call-is-not-clean", 0, "not_evaluated"),
        ("witnessed-breach-before-unknown-call", 3, "finding"),
        ("opaque-write-rhs", 1, "not_evaluated"),
    ] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path(), &[]);
        assert_eq!(output.status.code(), Some(2), "{id}: {}", stdout(&output));
        assert_eq!(report["mutation_analysis"]["coverage"], "incomplete");
        let function = function(&report);
        let evidence = &function["evidence"];
        assert_bound(&evidence["write_sites"], "lower_bound", writes);
        assert!(!evidence["boundaries"].as_array().unwrap().is_empty());
        assert_eq!(rule(function, "mutation_per_target")["outcome"], outcome);
        assert_eq!(
            rule(function, "mutated_targets")["outcome"],
            "not_evaluated"
        );
        assert_eq!(function["density"]["outcome"], "not_evaluated");
        if outcome == "finding" {
            assert!(report["mutation_analysis"]["findings"].as_u64().unwrap() > 0);
        }
    }
}

#[test]
fn closure_and_reference_boundaries_do_not_invent_immediate_writes_or_stale_roots() {
    for id in [
        "uninvoked-closure-body",
        "reassigned-reference-formal",
        "reborrow-alias",
        "macro-write-boundary",
    ] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path(), &[]);
        assert_eq!(output.status.code(), Some(2), "{id}: {}", stdout(&output));
        assert_eq!(report["mutation_analysis"]["coverage"], "incomplete");
        let function = function(&report);
        assert_eq!(function["density"]["outcome"], "not_evaluated");
        assert_eq!(
            rule(function, "mutation_per_target")["outcome"],
            "not_evaluated"
        );
        assert_eq!(
            rule(function, "mutated_targets")["outcome"],
            "not_evaluated"
        );
        assert!(
            !function["evidence"]["boundaries"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        if id == "uninvoked-closure-body" {
            assert!(function["evidence"]["sites"].as_array().unwrap().is_empty());
        }
    }
}

#[test]
fn mutation_reports_keep_full_compact_default_and_opt_in_failure_distinct() {
    let directory = workspace("borrowed-formal-pointee-writes");
    let (output, full) = requested(directory.path(), &[]);
    assert!(output.status.success());
    let (output, compact) = requested(directory.path(), &["--report-detail", "compact"]);
    assert!(output.status.success());
    assert_eq!(full["mutation_analysis"], compact["mutation_analysis"]);
    assert_eq!(full["matching"], compact["matching"]);
    let (output, default) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    assert_eq!(default["mutation_analysis"]["execution"], "not_requested");
    let (output, defaults) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--mutation-rules"],
    );
    assert!(output.status.success());
    assert_eq!(defaults["mutation_analysis"]["findings"], 0);
    for id in ["mutation_per_target", "mutated_targets"] {
        assert_eq!(rule(function(&defaults), id)["limit"], 3);
    }
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(report["mutation_analysis"]["findings"], 1);
}

#[test]
fn mutation_budget_exhaustion_is_not_a_clean_zero_population() {
    let directory = workspace("borrowed-formal-pointee-writes");
    let (output, report) = requested(directory.path(), &["--max-analysis-functions", "0"]);
    assert_eq!(output.status.code(), Some(2));
    let analysis = &report["mutation_analysis"];
    assert_eq!(analysis["execution"], "budget_exhausted");
    assert_eq!(analysis["coverage"], "incomplete");
    assert_eq!(analysis["assessed_functions"], 0);
    assert_eq!(analysis["expected_functions"], 1);
    assert_eq!(analysis["findings"], 0);
}

#[test]
fn strict_replay_reevaluates_mutation_policy_without_original_source_or_tools() {
    let directory = workspace("borrowed-formal-pointee-writes");
    let (output, original) = requested(directory.path(), &[]);
    assert!(output.status.success());
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let strict = replay(directory.path(), &original);
    assert_eq!(strict.status.code(), Some(1), "{}", stdout(&strict));
    let strict: Value = serde_json::from_slice(&strict.stdout).unwrap();
    policy(directory.path(), 3);
    let loose = replay(directory.path(), &original);
    assert!(loose.status.success(), "{}", stdout(&loose));
    let loose: Value = serde_json::from_slice(&loose.stdout).unwrap();
    assert_eq!(loose["verification_policy"], "strict-replay");
    assert_eq!(loose["mutation_analysis"]["findings"], 0);
    assert_eq!(function(&strict)["evidence"], function(&loose)["evidence"]);
    assert_eq!(loose["functions"], original["functions"]);
    assert_eq!(loose["matching"], original["matching"]);
    assert_eq!(
        loose["owned_source_inventory"],
        original["owned_source_inventory"]
    );
}

fn replay(directory: &Path, original: &Value) -> Output {
    let executable = cargo_bin("cargo-rstyle").canonicalize().unwrap();
    diagnostic_process::output(
        Command::new(executable)
            .current_dir(directory)
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                "hir-data-v3",
                "--mutation-rules",
                "--mutation-config",
                "mutation.toml",
                "--fail-on-findings",
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
            ])
            .env("PATH", directory.join("no-tools"))
            .env("CARGO", directory.join("no-cargo"))
            .env("RSTYLE_DRIVER", directory.join("no-driver")),
    )
    .unwrap()
}

#[test]
fn opaque_retargeting_and_tuple_shadowing_do_not_attribute_writes_to_stale_formals() {
    let sources = [
        "pub fn redirected<'a>(mut target:&'a mut u32,other:&'a mut u32) { macro_rules! redirect {()=>{target=other;}} redirect!(); *target=1; *target=2; *target=3; }",
        "pub fn raw_callback<'a>(mut target:&'a mut u32,other:&'a mut u32,callback:fn(*mut &'a mut u32,&'a mut u32)) { callback(&raw mut target,other); *target=1; *target=2; *target=3; }",
        "pub fn shadowed(target:&mut u32) { let (target,_)=(&mut 0u32,0); *target=1; *target=2; *target=3; }",
    ];
    for source in sources {
        let directory = clone_workspace();
        std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
        policy(directory.path(), 2);
        let (output, report) = requested(directory.path(), &[]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{source}: {}",
            stdout(&output)
        );
        assert_eq!(report["mutation_analysis"]["coverage"], "incomplete");
        assert_eq!(report["mutation_analysis"]["findings"], 0);
        let function = function(&report);
        assert_eq!(function["outcome"], "not_evaluated");
        let evidence = &function["evidence"];
        assert!(evidence["sites"].as_array().unwrap().is_empty());
        assert!(!evidence["boundaries"].as_array().unwrap().is_empty());
        assert_bound(&evidence["write_sites"], "lower_bound", 0);
        for id in ["mutation_per_target", "mutated_targets"] {
            assert_eq!(rule(function, id)["outcome"], "not_evaluated");
        }
    }
}
