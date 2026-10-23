//! Raw observations cannot activate; exact identity and preserved findings matter.
use super::*;

fn canonical() -> (HirFnFacts, HirNode) {
    let (facts, _, span) = crate::product_instances::test_support::conversion();
    let mut call = facts.body.children[0].children[2].clone();
    call.children[0].ty.as_mut().unwrap().normalized =
        Some(crate::data_types::token(&span).unwrap());
    (facts, call)
}

#[test]
fn raw_generic_facts_never_activate_a_product_instance() {
    let (facts, call) = canonical();
    assert!(!verify(&facts).unwrap());
    assert!(!source_findings(&facts).unwrap());
    assert!(!operation(&call, &facts).unwrap());
    assert!(summary(&call, &facts).unwrap().is_none());
}

#[test]
fn operation_requires_the_complete_saved_node_and_one_operand() {
    let (facts, expected) = canonical();
    assert!(checked_operation(&expected, &expected).unwrap());
    assert!(!checked_operation(&facts.body, &expected).unwrap());
    for mode in [
        "span", "snippet", "dispatch", "receiver", "missing", "extra",
    ] {
        let mut changed = expected.clone();
        match mode {
            "span" => changed.span.end += 1,
            "snippet" => changed.snippet.push(' '),
            "dispatch" => changed.dispatch = None,
            "receiver" => changed.children[0].res = None,
            "missing" => changed.children.clear(),
            "extra" => changed.children.push(expected.children[0].clone()),
            _ => unreachable!(),
        }
        assert!(checked_operation(&changed, &expected).is_err(), "{mode}");
    }
}

#[test]
fn identity_summary_keeps_every_nominal_component_and_has_no_hidden_effects() {
    let (_, mut call) = canonical();
    let summary = identity_summary(&call).unwrap();
    assert_eq!(summary.id, SUMMARY);
    assert_eq!(summary.argument_types, vec![summary.result_type.clone()]);
    assert!(!summary.memory_effects);
    assert!(!summary.mutates_arguments);
    assert!(!summary.may_panic);
    assert!(summary.always_returns);
    for token in ["usize", "unit"] {
        call.children[0].ty.as_mut().unwrap().normalized = Some(token.into());
        assert!(identity_summary(&call).is_err());
    }
    let (_, mut call) = canonical();
    let mut ty = crate::data_types::parse(&summary.result_type).unwrap();
    let crate::data_types::TypeToken::Struct { fields, .. } = &mut ty else {
        panic!("original Span must retain both nominal fields");
    };
    fields.swap(0, 1);
    call.children[0].ty.as_mut().unwrap().normalized = Some(ty.text().unwrap());
    assert!(identity_summary(&call).is_err());
}

#[test]
fn only_the_exact_original_deferred_conversion_finding_is_covered() {
    let (facts, call) = canonical();
    let before = facts.clone();
    findings(&facts, &call).unwrap();
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
            "snippet" => changed.source_diagnostics[0].snippet.push(' '),
            "severity" => changed.source_diagnostics[0].severity = "warning".into(),
            "reason" => changed.source_diagnostics[0].reason.clear(),
            _ => unreachable!(),
        }
        assert!(findings(&changed, &call).is_err(), "{mode}");
    }
}
