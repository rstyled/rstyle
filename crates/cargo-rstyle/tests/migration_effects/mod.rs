use super::*;
mod controls;

fn labels(id: &str) -> Value {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../eval/clones-capability/helper-effect-cases.json"
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
    directory
}

fn requested(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut arguments = vec!["--extraction-profile", "hir-data-v3", "--mutation-effects"];
    arguments.extend_from_slice(extra);
    clones(directory, &arguments)
}

fn function<'a>(report: &'a Value, suffix: &str) -> &'a Value {
    let qualified = format!("::{suffix}");
    let mut functions = report["mutation_effects"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|function| {
            let name = function["name"].as_str().unwrap();
            name == suffix || name.ends_with(&qualified)
        });
    let function = functions
        .next()
        .unwrap_or_else(|| panic!("missing function {suffix}"));
    assert!(functions.next().is_none(), "ambiguous {suffix}");
    function
}

fn slice<'a>(source: &'a str, span: &Value) -> &'a str {
    let start = span["start"].as_u64().unwrap() as usize;
    let end = span["end"].as_u64().unwrap() as usize;
    source
        .get(start..end)
        .unwrap_or_else(|| panic!("invalid span {span} in {source}"))
}

fn original_function<'a>(source: &'a str, function: &Value) -> &'a str {
    slice(source, &function["fn_span"])
}

fn summary<'a>(report: &'a Value, caller: &Value, call: &Value) -> &'a Value {
    let mut summaries = report["mutation_effects"]["summaries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|summary| {
            summary["unit"] == caller["unit"] && summary["summary_index"] == call["summary_index"]
        });
    let summary = summaries
        .next()
        .unwrap_or_else(|| panic!("missing summary: {call}"));
    assert!(summaries.next().is_none());
    summary
}

fn summary_owner<'a>(report: &'a Value, summary: &Value) -> &'a Value {
    report["mutation_effects"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| {
            function["unit"] == summary["unit"]
                && function["evidence"]["function_index"] == summary["function_index"]
        })
        .unwrap()
}

fn assert_root(root: &Value, source: &str) {
    assert!(!slice(source, &root["declaration_span"]).is_empty());
    assert!(!slice(source, &root["scope_span"]).is_empty());
    assert!(!root["native"].is_null());
    assert_eq!(root["provenance"], "formal_borrowed");
    assert!(root["formal"].as_u64().is_some());
}

fn assert_calls(report: &Value, source: &str) {
    let caller = function(report, "caller");
    let caller_source = original_function(source, caller);
    for call in caller["evidence"]["calls"].as_array().unwrap() {
        assert!(slice(caller_source, &call["span"]).contains('('));
        assert!(!call["native"].is_null());
        let summary = summary(report, caller, call);
        let callee = summary_owner(report, summary);
        let callee_source = original_function(source, callee);
        let sites = summary["sites"].as_array().unwrap();
        for mapping in call["mappings"].as_array().unwrap() {
            let index = mapping["summary_site_index"].as_u64().unwrap() as usize;
            let witness = &sites[index];
            assert!(slice(callee_source, &witness["span"]).contains('='));
            assert_eq!(witness["caller_visible"], true);
            assert_root(&witness["root"], callee_source);
            assert_root(&mapping["caller_root"], caller_source);
        }
    }
}

const POSITIVE_CASES: &[(&str, &str, u64, usize, usize, &str)] = &[
    (
        "owned-scalar-leaf-local-writes",
        "adjust",
        2,
        1,
        0,
        "none_proven",
    ),
    (
        "direct-borrowed-leaf-effect",
        "set",
        1,
        1,
        1,
        "writes_formal",
    ),
    ("same-helper-twice", "set", 1, 2, 1, "writes_formal"),
    (
        "leaf-multiple-writes-one-call",
        "set",
        3,
        1,
        1,
        "writes_formal",
    ),
    ("ordered-formal-mapping", "set", 1, 1, 1, "writes_formal"),
    ("nested-helper-not-yet-leaf", "leaf", 0, 1, 0, "none_proven"),
    ("distinct-caller-roots", "set", 1, 2, 2, "writes_formal"),
    (
        "same-name-different-definitions",
        "b::work",
        1,
        1,
        1,
        "writes_formal",
    ),
];

#[test]
fn original_leaf_calls_keep_direct_counts_and_caller_visible_effects_separate() {
    for &(id, leaf, direct, calls, roots, effect) in POSITIVE_CASES {
        let directory = workspace(id);
        let (output, report) = requested(directory.path(), &[]);
        assert!(output.status.success(), "{id}: {}", stdout(&output));
        assert_positive(
            &report,
            Positive {
                leaf,
                direct,
                calls,
                roots,
                effect,
            },
        );
        assert_calls(&report, labels(id)["source"].as_str().unwrap());
    }
}

struct Positive<'a> {
    leaf: &'a str,
    direct: u64,
    calls: usize,
    roots: usize,
    effect: &'a str,
}

fn assert_positive(report: &Value, expected: Positive<'_>) {
    let effects = &report["mutation_effects"];
    assert_eq!(effects["schema"], "rstyle.mutation-effects/3");
    assert_eq!(effects["execution"], "completed");
    assert_eq!(effects["coverage"], "complete");
    let caller = &function(report, "caller")["evidence"];
    assert_eq!(caller["coverage"], "complete");
    assert_eq!(caller["direct_site_count"], 0);
    assert_eq!(caller["calls"].as_array().unwrap().len(), expected.calls);
    assert_eq!(caller["roots"].as_array().unwrap().len(), expected.roots);
    assert_eq!(caller["caller_visible_mutation"], expected.effect);
    let leaf = &function(report, expected.leaf)["evidence"];
    assert_eq!(leaf["direct_site_count"], expected.direct);
    assert_eq!(leaf["coverage"], "complete");
    assert_eq!(leaf["caller_visible_mutation"], expected.effect);
}

#[test]
fn repeated_call_occurrences_share_one_summary_and_keep_exact_root_mapping() {
    let directory = workspace("same-helper-twice");
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success());
    let caller = function(&report, "caller");
    let calls = caller["evidence"]["calls"].as_array().unwrap();
    let statistics = &report["mutation_effects"]["units"][0]["statistics"];
    assert_eq!(statistics["observations"], 2);
    assert_eq!(statistics["leaf_checks"], 1);
    assert_eq!(statistics["calls"], 2);
    assert_eq!(calls.len(), 2);
    assert_ne!(calls[0]["span"], calls[1]["span"]);
    assert_ne!(calls[0]["call_index"], calls[1]["call_index"]);
    assert_eq!(calls[0]["summary_index"], calls[1]["summary_index"]);
    assert_eq!(
        calls[0]["mappings"][0]["caller_root"],
        calls[1]["mappings"][0]["caller_root"]
    );
    assert_eq!(
        report["mutation_effects"]["summaries"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        summary(&report, caller, &calls[0])["sites"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn argument_positions_map_written_callee_formals_to_original_caller_roots() {
    for (id, expected) in [
        ("ordered-formal-mapping", vec![0]),
        ("distinct-caller-roots", vec![0, 1]),
    ] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path(), &[]);
        assert!(output.status.success());
        let caller = function(&report, "caller");
        let mut formals: Vec<_> = caller["evidence"]["roots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|root| root["formal"].as_u64().unwrap())
            .collect();
        formals.sort();
        assert_eq!(formals, expected);
        if id == "ordered-formal-mapping" {
            let call = &caller["evidence"]["calls"][0];
            let selected = summary(&report, caller, call);
            let mapping = &call["mappings"][0];
            let index = mapping["summary_site_index"].as_u64().unwrap() as usize;
            assert_eq!(selected["sites"][index]["root"]["formal"], 1);
            assert_eq!(mapping["caller_root"]["formal"], 0);
        }
    }
}

#[test]
fn opaque_nonleaf_and_unsupported_argument_boundaries_never_become_clean_effects() {
    for id in [
        "external-call-unknown",
        "function-pointer-unknown",
        "drop-signature-unknown-even-unused",
        "mutable-reborrow-alias-unknown",
        "reference-redirection-unknown",
        "argument-evaluation-not-hidden",
        "unused-callback-formal-outside-signature-vocabulary",
        "recursive-cycle-not-leaf",
        "third-helper-edge-outside-summary-depth",
    ] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path(), &[]);
        assert_eq!(output.status.code(), Some(2), "{id}: {}", stdout(&output));
        let caller = &function(&report, "caller")["evidence"];
        assert_eq!(caller["coverage"], "incomplete", "{id}: {caller}");
        assert_eq!(
            caller["caller_visible_mutation"], "unknown",
            "{id}: {caller}"
        );
        assert!(caller["roots"].as_array().unwrap().is_empty(), "{id}");
        assert!(
            !caller["unresolved_boundaries"]
                .as_array()
                .unwrap()
                .is_empty()
                || !caller["reasons"].as_array().unwrap().is_empty(),
            "{id}"
        );
    }
}

#[test]
fn deferred_and_unsafe_bodies_remain_unknown() {
    let directory = workspace("closure-and-unsafe-not-leaves");
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    for name in ["deferred", "raw"] {
        let evidence = &function(&report, name)["evidence"];
        assert_eq!(evidence["coverage"], "incomplete");
        assert_eq!(evidence["caller_visible_mutation"], "unknown");
    }
}

#[test]
fn nominal_coercions_and_shared_references_have_no_mutable_primitive_authority() {
    for id in [
        "nominal-u32-deref-coercion-not-primitive-formal",
        "shared-u32-not-mutable-formal",
    ] {
        let directory = workspace(id);
        let (output, report) = requested(directory.path(), &[]);
        assert_eq!(output.status.code(), Some(2), "{id}: {}", stdout(&output));
        let caller = &function(&report, "caller")["evidence"];
        assert_eq!(caller["coverage"], "incomplete", "{id}: {caller}");
        assert_eq!(
            caller["caller_visible_mutation"], "unknown",
            "{id}: {caller}"
        );
        assert!(caller["roots"].as_array().unwrap().is_empty(), "{id}");
        assert!(formal_type_observation(&report, "caller").is_null(), "{id}");
        if id == "nominal-u32-deref-coercion-not-primitive-formal" {
            let observation = formal_type_observation(&report, "set");
            assert_eq!(observation["identity"], "rstyle.consumer-formal-types/1");
            assert_eq!(observation["mutable_u32"].as_array().unwrap().len(), 1);
            assert_eq!(observation["mutable_u32"][0]["parameter"], 0);
        }
    }
}

fn formal_type_observation<'a>(report: &'a Value, name: &str) -> &'a Value {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| {
            let captured = function["facts"]["name"].as_str().unwrap();
            captured == name || captured.ends_with(&format!("::{name}"))
        })
        .unwrap();
    &function["facts"]["build"]["frontend_options"]["consumer_formal_types"]
}

#[test]
fn a_verified_call_witness_survives_an_unknown_callback_remainder() {
    let id = "witness-survives-unknown-remainder";
    let directory = workspace(id);
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let evidence = &function(&report, "caller")["evidence"];
    assert_eq!(evidence["coverage"], "incomplete");
    assert_eq!(
        evidence["caller_visible_mutation"],
        "witnessed_writes_formal_with_unknown_remainder"
    );
    assert_eq!(evidence["direct_site_count"], 0);
    assert_eq!(evidence["calls"].as_array().unwrap().len(), 1);
    assert_eq!(evidence["roots"].as_array().unwrap().len(), 1);
    assert!(
        !evidence["unresolved_boundaries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_calls(&report, labels(id)["source"].as_str().unwrap());
}

#[test]
fn effects_are_independently_requested_and_full_compact_reports_agree() {
    let directory = workspace("direct-borrowed-leaf-effect");
    let (output, full) = requested(directory.path(), &[]);
    assert!(output.status.success());
    let (output, compact) = requested(directory.path(), &["--report-detail", "compact"]);
    assert!(output.status.success());
    assert_eq!(full["mutation_effects"], compact["mutation_effects"]);
    assert_eq!(full["mutation_analysis"]["execution"], "not_requested");
    let (output, default) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    assert_eq!(default["mutation_effects"]["execution"], "not_requested");
    let (output, rules) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--mutation-rules"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(rules["mutation_effects"]["execution"], "not_requested");
}

#[test]
fn omitted_function_owners_are_budget_exhausted_not_clean() {
    let directory = workspace("direct-borrowed-leaf-effect");
    let (output, report) = requested(directory.path(), &["--max-analysis-functions", "0"]);
    assert_eq!(output.status.code(), Some(2));
    let effects = &report["mutation_effects"];
    assert_eq!(effects["execution"], "budget_exhausted");
    assert_eq!(effects["coverage"], "incomplete");
    assert_eq!(effects["assessed_functions"], 0);
    assert_eq!(effects["expected_functions"], 2);
    assert_eq!(effects["functions"].as_array().unwrap().len(), 2);
    for function in effects["functions"].as_array().unwrap() {
        let evidence = &function["evidence"];
        assert_eq!(evidence["execution"], "budget_exhausted");
        assert_eq!(evidence["coverage"], "incomplete");
        assert_eq!(evidence["caller_visible_mutation"], "unknown");
    }
}

#[test]
fn strict_effect_replay_preserves_witnesses_without_original_tree_or_tools() {
    for id in [
        "owned-scalar-leaf-local-writes",
        "direct-borrowed-leaf-effect",
    ] {
        let directory = workspace(id);
        let (output, original) = requested(directory.path(), &[]);
        assert!(output.status.success());
        std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
        std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
        for detail in ["full", "compact"] {
            let output = replay(directory.path(), &original, detail);
            assert!(output.status.success(), "{id}: {}", stdout(&output));
            let replay: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(replay["verification_policy"], "strict-replay");
            assert_eq!(replay["mutation_effects"], original["mutation_effects"]);
            assert_eq!(
                replay["owned_source_inventory"],
                original["owned_source_inventory"]
            );
            assert_eq!(replay["matching"], original["matching"]);
        }
    }
}

fn replay(directory: &Path, original: &Value, detail: &str) -> Output {
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
                "--mutation-effects",
                "--report-detail",
                detail,
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
            ])
            .env("PATH", directory.join("no-tools"))
            .env("CARGO", directory.join("no-cargo"))
            .env("RSTYLE_DRIVER", directory.join("no-driver")),
    )
    .unwrap()
}
