//! Retained compiler calls and actual post-scan findings with adversarial edits.
use super::*;
use serde_json::Value;

fn canonical() -> HirFnFacts {
    let operations: Value =
        serde_json::from_str(include_str!("../operations/fixtures.json")).unwrap();
    serde_json::from_value(operations[0]["canonical"].clone()).unwrap()
}

fn call(facts: &HirFnFacts) -> &HirNode {
    &facts.body.children[0].children[1]
}

#[test]
fn raw_generic_facts_and_observations_never_activate_an_instance() {
    let facts = canonical();
    assert!(!verify(&facts).unwrap());
    assert!(!source_findings(&facts).unwrap());
    assert!(operation(call(&facts), &facts).unwrap().is_none());
}

#[test]
fn retained_conversion_has_one_exact_operand_and_shared_formal_origin() {
    let facts = canonical();
    let node = call(&facts);
    let checked = checked_operation(node, node).unwrap().unwrap();
    assert_eq!(checked.receiver, &node.children[0]);
    assert_eq!(checked.origin.parameter, 0);
    assert_eq!(checked.origin.path, [Projection::SharedReferent]);
    assert!(checked_operation(&facts.body, node).unwrap().is_none());
}

#[test]
fn same_site_edits_cannot_reuse_the_checked_operation() {
    let facts = canonical();
    let expected = call(&facts);
    for mode in [
        "snippet",
        "span",
        "resolution",
        "dispatch",
        "operand",
        "missing",
        "duplicate",
    ] {
        let mut changed = expected.clone();
        match mode {
            "snippet" => changed.snippet.push(' '),
            "span" => changed.span.start += 1,
            "resolution" => changed.res = None,
            "dispatch" => changed.dispatch = None,
            "operand" => changed.children[0].res = None,
            "missing" => changed.children.clear(),
            "duplicate" => changed.children.push(expected.children[0].clone()),
            _ => unreachable!(),
        }
        assert!(checked_operation(&changed, expected).is_err(), "{mode}");
    }
}

#[test]
fn exact_retained_deferred_finding_is_covered_without_rewriting_it() {
    let scans: Value = serde_json::from_str(include_str!("scan-fixtures.json")).unwrap();
    for scan in scans.as_array().unwrap() {
        let mut facts = canonical();
        facts.source_scan_complete = scan["source_scan_complete"].as_bool().unwrap();
        facts.source_diagnostics =
            serde_json::from_value(scan["source_diagnostics"].clone()).unwrap();
        let before = facts.clone();
        findings(&facts, call(&facts)).unwrap();
        assert_eq!(facts, before);
        for mode in [
            "absent",
            "extra",
            "complete",
            "capability",
            "construct",
            "span",
            "snippet",
            "severity",
            "reason",
        ] {
            let mut changed = facts.clone();
            match mode {
                "absent" => changed.source_diagnostics.clear(),
                "extra" => changed
                    .source_diagnostics
                    .push(facts.source_diagnostics[0].clone()),
                "complete" => changed.source_scan_complete = true,
                "capability" => changed.source_diagnostics[0].capability = "rust.macros".into(),
                "construct" => changed.source_diagnostics[0].construct = "expr_call".into(),
                "span" => changed.source_diagnostics[0].span.end += 1,
                "snippet" => changed.source_diagnostics[0].snippet = "other.as_ref()".into(),
                "severity" => changed.source_diagnostics[0].severity = "warning".into(),
                "reason" => changed.source_diagnostics[0].reason.clear(),
                _ => unreachable!(),
            }
            assert!(findings(&changed, call(&facts)).is_err(), "{mode}");
        }
    }
}
