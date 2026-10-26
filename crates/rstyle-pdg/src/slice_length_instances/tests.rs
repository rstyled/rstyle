//! Independent fixed compiler HIR plus mutations, without a producer equality oracle.
use super::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Sample {
    name: String,
    fn_span: crate::ByteSpan,
    source: crate::SourceFacts,
    signature: crate::SignatureFacts,
    bindings: Vec<crate::BindingFact>,
    body: HirNode,
}

fn original() -> HirFnFacts {
    let mut facts: HirFnFacts = serde_json::from_str(include_str!(
        "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    let sample: Sample = serde_json::from_str(include_str!("body_fixture.json")).unwrap();
    facts.name = sample.name;
    facts.fn_span = sample.fn_span;
    facts.source = sample.source;
    facts.signature = sample.signature;
    facts.bindings = sample.bindings;
    facts.body = sample.body;
    facts
}

#[test]
fn complete_original_guard_has_exact_roles_and_formal_roots() {
    let original = original();
    let source = source::verify(&original.source.bytes).unwrap();
    let roles = body::verify(&original, &source).unwrap();
    assert_eq!(roles.values.len(), 16);
    assert_eq!(roles.values[&site(&roles.condition)], body::Role::Bool);
    assert_eq!(roles.values[&site(&roles.length)], body::Role::Usize);
    assert_eq!(roles.values[&site(&roles.returned)], body::Role::Returned);
}

#[test]
fn original_guard_rejects_hidden_operations_and_rebound_formals() {
    for mutation in [
        "receiver",
        "what",
        "operation",
        "duplicate",
        "extra_child",
        "binding",
    ] {
        let mut facts = original();
        let source = source::verify(&facts.source.bytes).unwrap();
        let selection = &mut facts.body.children[0].children[0];
        match mutation {
            "receiver" => {
                selection.children[0].children[0].children[0]
                    .res
                    .as_mut()
                    .unwrap()
                    .target = Some(facts.signature.params[1].pat.hir.clone())
            }
            "what" => {
                selection.children[1].children[0].children[0].children[0].children[0].children[0]
                    .res
                    .as_mut()
                    .unwrap()
                    .target = Some(facts.signature.params[0].pat.hir.clone())
            }
            "operation" => {
                selection.res = Some(crate::PathRes {
                    kind: "hidden".into(),
                    target: None,
                    description: "hidden call".into(),
                })
            }
            "duplicate" => {
                selection.children[0].children[1].hir =
                    selection.children[0].children[0].children[0].hir.clone()
            }
            "extra_child" => selection.children.push(selection.children[0].clone()),
            "binding" => facts.bindings[0].is_mut = true,
            _ => unreachable!(),
        }
        assert!(body::verify(&facts, &source).is_err(), "{mutation}");
    }
}

#[test]
fn concrete_substitution_requires_structural_u8_and_complete_single_argument() {
    let input = serde_json::json!([{
        "kind":"type", "value":"display text is not authority", "display_only":true,
        "identity_status":"known", "identity":{"kind":"builtin","name":"u8"}
    }]);
    let arguments: Vec<transport::Argument> = serde_json::from_value(input.clone()).unwrap();
    types::arguments_match(&arguments, &[types::builtin("u8")]).unwrap();
    for mutation in ["width", "status", "kind", "count"] {
        let mut changed = input.clone();
        match mutation {
            "width" => {
                changed[0]["identity"]["name"] = serde_json::json!("u16");
                changed[0]["value"] = serde_json::json!("u8");
            }
            "status" => changed[0]["identity_status"] = serde_json::json!("unsupported"),
            "kind" => changed[0]["kind"] = serde_json::json!("const"),
            "count" => changed.as_array_mut().unwrap().clear(),
            _ => unreachable!(),
        }
        let arguments: Vec<transport::Argument> = serde_json::from_value(changed).unwrap();
        assert!(
            types::arguments_match(&arguments, &[types::builtin("u8")]).is_err(),
            "{mutation}"
        );
    }
}

fn deferred_finding(call: &HirNode) -> crate::HirDiagnostic {
    crate::HirDiagnostic {
        capability: "rust.call-summaries".into(),
        construct: "expr_method_call".into(),
        span: call.span,
        snippet: call.snippet.clone(),
        severity: "error".into(),
        reason: "Original owned source contains deferred rust.call-summaries; cfg removal cannot certify a supported remainder.".into(),
    }
}

#[test]
fn exact_deferred_finding_accepts_both_scan_flags_and_preserves_legacy_requirement() {
    let mut facts = original();
    let source = source::verify(&facts.source.bytes).unwrap();
    let roles = body::verify(&facts, &source).unwrap();
    let call = receipts::find(&facts.body, &roles.length).unwrap().clone();
    facts.source_diagnostics = vec![deferred_finding(&call)];
    for complete in [false, true] {
        facts.source_scan_complete = complete;
        findings(&facts, &roles.length).unwrap();
        assert_eq!(
            crate::source_evidence::checked_activation::deferred_call(&facts, &call).is_ok(),
            !complete
        );
    }
    facts.source_diagnostics.clear();
    findings(&facts, &roles.length).unwrap();
    facts.source_scan_complete = false;
    assert!(findings(&facts, &roles.length).is_err());
}

#[test]
fn exact_deferred_finding_rejects_changed_fields_and_roster() {
    let mut facts = original();
    let source = source::verify(&facts.source.bytes).unwrap();
    let roles = body::verify(&facts, &source).unwrap();
    let call = receipts::find(&facts.body, &roles.length).unwrap().clone();
    for complete in [false, true] {
        facts.source_scan_complete = complete;
        for mutation in [
            "site",
            "capability",
            "construct",
            "reason",
            "severity",
            "snippet",
            "duplicate",
        ] {
            facts.source_diagnostics = vec![deferred_finding(&call)];
            let finding = &mut facts.source_diagnostics[0];
            match mutation {
                "site" => finding.span.start += 1,
                "capability" => finding.capability.push('x'),
                "construct" => finding.construct.push('x'),
                "reason" => finding.reason.push('x'),
                "severity" => finding.severity = "warning".into(),
                "snippet" => finding.snippet.push(' '),
                "duplicate" => facts.source_diagnostics.push(deferred_finding(&call)),
                _ => unreachable!(),
            }
            assert!(
                findings(&facts, &roles.length).is_err(),
                "{complete}: {mutation}"
            );
        }
    }
}
