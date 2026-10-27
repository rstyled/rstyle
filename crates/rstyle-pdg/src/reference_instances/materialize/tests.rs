//! Independent normalization expectations; these do not mint checked authority.
use super::*;

fn canonical(index: usize) -> HirFnFacts {
    let operations: Value =
        serde_json::from_str(include_str!("../operations/fixtures.json")).unwrap();
    let bodies: Value = serde_json::from_str(include_str!("../body/fixtures.json")).unwrap();
    let fixture = &bodies[index];
    let mut facts: HirFnFacts =
        serde_json::from_value(operations[index]["canonical"].clone()).unwrap();
    facts.signature = serde_json::from_value(fixture["canonical_signature"].clone()).unwrap();
    facts.bindings = serde_json::from_value(fixture["canonical_bindings"].clone()).unwrap();
    facts.body = serde_json::from_value(fixture["canonical_body"].clone()).unwrap();
    facts
}

#[test]
fn concrete_formal_types_preserve_source_findings_and_every_nonformal_value() {
    for (index, haystack) in [Haystack::Bytes, Haystack::Str].into_iter().enumerate() {
        let original = canonical(index);
        let projected = concrete_facts(&original, haystack).unwrap();
        let token: TypeToken = serde_json::from_str(
            projected.signature.params[0]
                .ty
                .normalized
                .as_deref()
                .unwrap(),
        )
        .unwrap();
        let expected = if haystack == Haystack::Str {
            TypeToken::Shared(Box::new(TypeToken::Str))
        } else {
            TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
                "u8".into(),
            )))))
        };
        assert_eq!(token, expected);
        assert_only_formal_normalization_changed(&original, projected);
    }
}

fn assert_only_formal_normalization_changed(original: &HirFnFacts, mut projected: HirFnFacts) {
    let formal = &original.signature.params[0].pat.hir;
    projected.signature.params[0].ty.normalized =
        original.signature.params[0].ty.normalized.clone();
    restore_node(
        &original.signature.params[0].pat,
        &mut projected.signature.params[0].pat,
        formal,
    );
    restore_node(&original.body, &mut projected.body, formal);
    for (before, after) in original.bindings.iter().zip(&mut projected.bindings) {
        if &before.hir == formal {
            after.ty.normalized = before.ty.normalized.clone();
        } else {
            assert_eq!(before.ty.normalized, after.ty.normalized);
        }
    }
    assert_eq!(*original, projected);
}

fn restore_node(before: &HirNode, after: &mut HirNode, formal: &HirRef) {
    let formal_read = before.res.as_ref().is_some_and(|resolution| {
        resolution.kind == "local" && resolution.target.as_ref() == Some(formal)
    });
    if &before.hir == formal || formal_read {
        after.ty.as_mut().unwrap().normalized = before.ty.as_ref().unwrap().normalized.clone();
    } else {
        assert_eq!(before.ty, after.ty);
    }
    for (before, after) in before.children.iter().zip(&mut after.children) {
        restore_node(before, after, formal);
    }
}

#[test]
fn generic_source_and_conversion_resolution_survive_concretization() {
    let original = canonical(0);
    let projected = concrete_facts(&original, Haystack::Bytes).unwrap();
    assert_eq!(projected.signature.params[0].ty.spelling, "&'h H");
    assert_eq!(projected.signature.params[0].ty.observed, "&H");
    assert_eq!(projected.signature.generics, original.signature.generics);
    let before = &original.body.children[0].children[1];
    let after = &projected.body.children[0].children[1];
    assert_eq!(before.res, after.res);
    assert_eq!(before.dispatch, after.dispatch);
}

#[test]
fn absent_formal_type_or_invented_arity_cannot_be_materialized() {
    let mut facts = canonical(0);
    facts.signature.params[0].pat.ty = None;
    assert!(concrete_facts(&facts, Haystack::Str).is_err());
    let mut facts = canonical(0);
    facts
        .signature
        .params
        .push(facts.signature.params[0].clone());
    assert!(concrete_facts(&facts, Haystack::Bytes).is_err());
}

#[test]
fn observation_failure_cannot_create_a_checked_token() {
    let facts = canonical(0);
    assert!(verify(&facts, &serde_json::json!({}), Haystack::Bytes).is_err());
}

#[test]
fn normalization_cannot_erase_frontend_findings_or_claim_a_complete_scan() {
    let mut facts = canonical(0);
    facts.source_scan_complete = false;
    facts.source_diagnostics.push(crate::HirDiagnostic {
        capability: "rust.generics".into(),
        construct: "generic_parameter".into(),
        span: crate::ByteSpan { start: 11, end: 12 },
        snippet: "H".into(),
        severity: "error".into(),
        reason: "Original unsupported generic owner".into(),
    });
    let projected = concrete_facts(&facts, Haystack::Bytes).unwrap();
    assert!(!projected.source_scan_complete);
    assert_eq!(projected.source_diagnostics, facts.source_diagnostics);
    assert_only_formal_normalization_changed(&facts, projected);
}
