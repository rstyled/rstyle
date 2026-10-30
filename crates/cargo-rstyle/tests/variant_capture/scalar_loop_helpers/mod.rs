//! Complete scalar loop/caller composition; assertions and exits remain evidence.
use super::float_kernels::function;
use super::*;
use rstyle_pdg::unit_store::{BundleManifest, UnitStore, transport_checksum};

const SELECTION: &str = include_str!(
    "../../../../../tests/fixtures/function-families/scalar-loop-callee-selection.json"
);

fn original() -> String {
    let selection: Value = serde_json::from_str(SELECTION).unwrap();
    selection["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|function| function["source"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

fn source() -> String {
    let original = original();
    let alpha = original
        .replace("pow5factor_32", "alpha_factor")
        .replace("multiple_of_power_of_5_32", "alpha_predicate")
        .replace("value", "input")
        .replace("count", "exponent")
        .replace("let q", "let quotient")
        .replace("value = q", "value = quotient")
        .replace("input = q", "input = quotient")
        .replace("let r", "let remainder")
        .replace("if r !=", "if remainder !=");
    format!(
        "{original}\n{alpha}\npub fn wrapper(value:u32,p:u32)->bool {{ multiple_of_power_of_5_32(value,p) }}\npub fn alpha_wrapper(input:u32,p:u32)->bool {{ alpha_predicate(input,p) }}\n"
    )
}

fn capture(source: &str, debug: bool) -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    if !debug {
        let manifest = directory.path().join("Cargo.toml");
        let mut bytes = std::fs::read_to_string(&manifest).unwrap();
        bytes.push_str("\n[profile.dev]\ndebug-assertions = false\n");
        std::fs::write(manifest, bytes).unwrap();
    }
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn bounded_scalar_loops_compose_with_callers_and_alpha_peers() {
    let (_directory, report) = capture(&source(), true);
    for name in [
        "pow5factor_32",
        "multiple_of_power_of_5_32",
        "alpha_factor",
        "alpha_predicate",
        "wrapper",
        "alpha_wrapper",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    let store = super::word_classifiers::verify_admissions(&report);
    let analysis = store.analysis();
    for entry in report["functions"].as_array().unwrap() {
        let index = shared_evidence::index(entry);
        let facts = analysis.function(index).unwrap();
        rstyle_pdg::producer::verify(store.pair(index).unwrap(), &facts).unwrap();
    }
    accepted_pair(&report, "pow5factor_32", "alpha_factor");
    accepted_pair(&report, "multiple_of_power_of_5_32", "alpha_predicate");
    accepted_pair(&report, "wrapper", "alpha_wrapper");
    reject_corruptions(&report);
}

fn accepted_pair(report: &Value, first: &str, second: &str) {
    let first = &function(report, first)["key"];
    let second = &function(report, second)["key"];
    let pair = report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            (&pair["first"] == first && &pair["second"] == second)
                || (&pair["first"] == second && &pair["second"] == first)
        })
        .expect("complete alpha peer comparison");
    assert_eq!(pair["decision"], "Accepted", "{pair}");
}

fn reject_corruptions(report: &Value) {
    let bundle = shared_evidence::wire(function(report, "pow5factor_32"));
    // All attacks share one immutable unit; validate its positive baseline once.
    UnitStore::from_bundle(&serde_json::to_vec(&bundle).unwrap()).unwrap();
    reject_caller_corruptions(&bundle, function(report, "multiple_of_power_of_5_32"));
    let helper = function(report, "pow5factor_32");
    for mutation in ["debug", "unknown_debug", "overflow", "graph", "order"] {
        reject_resealed(&bundle, helper, |bundle, index| {
            corrupt_loop_evidence(bundle, index, mutation);
        });
    }
    reject_resealed(&bundle, function(report, "wrapper"), downgrade_policy);
}

fn reject_caller_corruptions(original: &Value, caller: &Value) {
    for mutation in [
        "panic",
        "returns",
        "evaluation",
        "dependence",
        "binding",
        "diverges",
        "backedges",
        "identity",
        "missing_bound",
        "policy",
    ] {
        reject_resealed(original, caller, |bundle, index| {
            let callee = bundle["links"][index][0]["function"].as_u64().unwrap() as usize;
            let helper = bundle["certified"][callee].as_u64().unwrap() as usize;
            let summary = &mut bundle["helpers"][helper]["summary"];
            assert_eq!(summary["identity"], "rstyle.helper.bounded-factor-loop/1");
            assert_eq!(summary["bounded_loop"]["may_diverge"], false);
            assert_eq!(summary["bounded_loop"]["positive_max_backedges"], 13);
            if mutation == "binding" {
                bundle["links"][index][0]["function"] = index.into();
            } else if mutation == "policy" {
                let graph = bundle["roots"][index].as_u64().unwrap() as usize;
                bundle["graphs"][graph]["pdg"]["profile"]["version"] = "3.75.0".into();
                bundle["graphs"][graph]["cfg"]["profile"]["version"] = "3.75.0".into();
            } else {
                corrupt_summary(summary, mutation);
            }
        });
    }
}

fn corrupt_summary(summary: &mut Value, mutation: &str) {
    match mutation {
        "panic" => {
            assert_eq!(summary["may_panic"], true);
            summary["may_panic"] = false.into();
        }
        "returns" => {
            assert_eq!(summary["always_returns"], false);
            summary["always_returns"] = true.into();
        }
        "evaluation" => summary["argument_evaluation"] = "unordered".into(),
        "dependence" => summary["result_dependence"] = "constant".into(),
        "diverges" => summary["bounded_loop"]["may_diverge"] = true.into(),
        "backedges" => summary["bounded_loop"]["positive_max_backedges"] = 12.into(),
        "identity" => summary["identity"] = "rstyle.helper.source-body/1".into(),
        "missing_bound" => {
            summary.as_object_mut().unwrap().remove("bounded_loop");
        }
        _ => unreachable!(),
    }
}

fn corrupt_loop_evidence(bundle: &mut Value, index: usize, mutation: &str) {
    let graph = bundle["roots"][index].as_u64().unwrap() as usize;
    match mutation {
        "debug" => bundle["functions"][index]["build"]["debug_assertions"]["value"] = false.into(),
        "unknown_debug" => {
            bundle["functions"][index]["build"]["debug_assertions"]["status"] = "unknown".into()
        }
        "overflow" => {
            bundle["functions"][index]["build"]["overflow_checks"]["value"] = false.into()
        }
        "graph" => {
            let edges = bundle["graphs"][graph]["cfg"]["edges"]
                .as_array_mut()
                .unwrap();
            assert!(!edges.is_empty());
            edges.pop();
        }
        "order" => {
            let edge = bundle["graphs"][graph]["pdg"]["edges"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|edge| {
                    edge["subkind"] == "value_operand" && edge["operand_position"].is_number()
                })
                .expect("ordered arithmetic operand");
            edge["operand_position"] = 99.into();
        }
        _ => unreachable!(),
    }
}

fn downgrade_policy(bundle: &mut Value, index: usize) {
    let graph = bundle["roots"][index].as_u64().unwrap() as usize;
    bundle["graphs"][graph]["pdg"]["profile"]["version"] = "3.75.0".into();
    bundle["graphs"][graph]["cfg"]["profile"]["version"] = "3.75.0".into();
}

// Resealing makes every rejection exercise evidence checks, not stale byte seals.
fn reject_resealed(original: &Value, entry: &Value, edit: impl FnOnce(&mut Value, usize)) {
    let mut bundle = original.clone();
    edit(&mut bundle, shared_evidence::index(entry));
    let bytes = serde_json::to_vec(&bundle).unwrap();
    let manifest = BundleManifest {
        schema: "rstyle.shared-unit.manifest/2".into(),
        checksum: transport_checksum(&bytes),
        bytes: bytes.len() as u64,
    };
    manifest.verify(&bytes).unwrap();
    assert!(UnitStore::from_bundle(&bytes).is_err());
    assert!(rstyle_pdg::verification_policy::generated(|| UnitStore::from_bundle(&bytes)).is_err());
}

fn altered_helpers() -> Vec<(&'static str, String)> {
    let selection: Value = serde_json::from_str(SELECTION).unwrap();
    let helper = selection["functions"][0]["source"].as_str().unwrap();
    vec![
        (
            "divisor",
            helper.replace(" / 5", " / 3").replace(" % 5", " % 3"),
        ),
        ("mixed_divisor", helper.replace(" % 5", " % 3")),
        (
            "changed_return",
            helper.replace("\n    count\n", "\n    count + 1\n"),
        ),
        (
            "changed_update",
            helper.replace("value = q;", "value = q + 1;"),
        ),
        (
            "missing_assertion",
            helper.replace("        debug_assert!(value != 0);\n", ""),
        ),
        (
            "extra_statement",
            helper.replace(
                "    let mut count",
                "    let _extra = value + 1;\n    let mut count",
            ),
        ),
    ]
}

fn unsupported_source() -> (String, Vec<&'static str>) {
    let mut source = String::new();
    let mut names = Vec::new();
    for (name, body) in altered_helpers() {
        source.push_str(&body.replace("pow5factor_32", name));
        source.push_str(&format!(
            "\npub fn call_{name}(value: u32) -> bool {{ {name}(value) >= 1 }}\n"
        ));
        names.push(name);
    }
    source.push_str(
        r#"
fn unbounded(value: u32) -> u32 { loop { if value == 0 { break; } } value }
pub fn call_unbounded(value: u32) -> u32 { unbounded(value) }
fn recursive(value: u32) -> u32 { if value == 0 { 0 } else { recursive(value - 1) } }
pub fn call_recursive(value: u32) -> u32 { recursive(value) }
"#,
    );
    names.extend(["unbounded", "recursive"]);
    (source, names)
}

#[test]
fn altered_or_unbounded_loops_do_not_acquire_helper_certification() {
    let (source, names) = unsupported_source();
    let (_directory, report) = capture(&source, true);
    let bundle = shared_evidence::wire(&report["functions"][0]);
    for name in names {
        let helper = function(&report, name);
        assert_eq!(
            bundle["certified"][shared_evidence::index(helper)],
            Value::Null,
            "{name}"
        );
        let caller_name = format!("call_{name}");
        let caller = function(&report, &caller_name);
        assert_eq!(
            caller["outcome"], "excluded",
            "{caller_name}: {}",
            caller["reasons"]
        );
    }
}

#[test]
fn disabled_debug_assertions_do_not_certify_the_zero_diverging_loop() {
    let (_directory, report) = capture(&original(), false);
    let helper = function(&report, "pow5factor_32");
    assert_eq!(helper["facts"]["build"]["debug_assertions"]["value"], false);
    let bundle = shared_evidence::wire(helper);
    assert_eq!(
        bundle["certified"][shared_evidence::index(helper)],
        Value::Null
    );
    let caller = function(&report, "multiple_of_power_of_5_32");
    assert_eq!(caller["outcome"], "excluded", "{}", caller["reasons"]);
}
