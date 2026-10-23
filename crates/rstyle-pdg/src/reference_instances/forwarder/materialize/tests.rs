//! Independent concrete type expectations preserve every original source fact.
use super::*;
use crate::data_types::TypeToken;
use serde_json::Value;

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!("../fixtures.json")).unwrap()
}

fn original(index: usize) -> HirFnFacts {
    serde_json::from_value(fixtures()[index]["original"].clone()).unwrap()
}

fn result() -> String {
    let values: Value = serde_json::from_str(include_str!("../../body/fixtures.json")).unwrap();
    values[0]["canonical_signature"]["return_ty"]["normalized"]
        .as_str()
        .unwrap()
        .into()
}

#[test]
fn exact_formal_reads_and_complete_tail_results_receive_concrete_types() {
    for (index, haystack) in [Haystack::Bytes, Haystack::Str].into_iter().enumerate() {
        let projected = concrete_facts(&original(index), haystack, &result()).unwrap();
        let referent = if index == 0 {
            TypeToken::Slice(Box::new(TypeToken::Scalar("u8".into())))
        } else {
            TypeToken::Str
        };
        let input = TypeToken::Shared(Box::new(referent));
        for ty in [
            &projected.signature.params[0].ty,
            projected.signature.params[0].pat.ty.as_ref().unwrap(),
            &projected.bindings[0].ty,
            projected.body.children[0].children[0].ty.as_ref().unwrap(),
        ] {
            assert_eq!(
                crate::data_types::parse(ty.normalized.as_deref().unwrap()).unwrap(),
                input
            );
        }
        assert_result_sites(&projected);
    }
}

fn assert_result_sites(facts: &HirFnFacts) {
    for ty in [
        &facts.signature.return_ty,
        facts.body.ty.as_ref().unwrap(),
        facts.body.children[0].ty.as_ref().unwrap(),
    ] {
        assert_eq!(ty.normalized.as_deref(), Some(result().as_str()));
    }
    assert!(facts.body.children[0].res.is_none());
    assert!(facts.body.children[0].dispatch.is_none());
}

#[test]
fn normalization_preserves_source_findings_and_every_non_type_field() {
    for (index, haystack) in [Haystack::Bytes, Haystack::Str].into_iter().enumerate() {
        let mut source = original(index);
        source.source_scan_complete = false;
        source.source_diagnostics.push(crate::HirDiagnostic {
            capability: "rust.generics".into(),
            construct: "generic_parameter".into(),
            span: crate::ByteSpan { start: 18, end: 23 },
            snippet: "&'h H".into(),
            severity: "error".into(),
            reason: "Preserved original unsupported finding".into(),
        });
        let mut projected = concrete_facts(&source, haystack, &result()).unwrap();
        restore_normalized(&source, &mut projected);
        assert_eq!(source, projected);
    }
}

fn restore_normalized(source: &HirFnFacts, projected: &mut HirFnFacts) {
    projected.signature.params[0].ty.normalized = source.signature.params[0].ty.normalized.clone();
    projected.signature.return_ty.normalized = source.signature.return_ty.normalized.clone();
    projected.bindings[0].ty.normalized = source.bindings[0].ty.normalized.clone();
    restore_node(
        &source.signature.params[0].pat,
        &mut projected.signature.params[0].pat,
    );
    restore_node(&source.body, &mut projected.body);
}

fn restore_node(source: &HirNode, projected: &mut HirNode) {
    projected.ty.as_mut().unwrap().normalized = source.ty.as_ref().unwrap().normalized.clone();
    for (before, after) in source.children.iter().zip(&mut projected.children) {
        restore_node(before, after);
    }
}

#[test]
fn conflicting_result_types_cannot_be_erased_by_constructor_normalization() {
    for role in ["signature", "block", "call"] {
        let mut facts = original(0);
        let ty = match role {
            "signature" => &mut facts.signature.return_ty,
            "block" => facts.body.ty.as_mut().unwrap(),
            _ => facts.body.children[0].ty.as_mut().unwrap(),
        };
        ty.normalized = Some("u8".into());
        assert!(
            concrete_facts(&facts, Haystack::Bytes, &result()).is_err(),
            "{role}"
        );
    }
}

#[test]
fn missing_type_or_invented_argument_cannot_be_materialized() {
    for role in ["formal", "read", "call", "block", "argument"] {
        let mut facts = original(0);
        match role {
            "formal" => facts.signature.params[0].pat.ty = None,
            "read" => facts.body.children[0].children[0].ty = None,
            "call" => facts.body.children[0].ty = None,
            "block" => facts.body.ty = None,
            _ => facts.body.children[0]
                .children
                .push(facts.signature.params[0].pat.clone()),
        }
        assert!(
            concrete_facts(&facts, Haystack::Bytes, &result()).is_err(),
            "{role}"
        );
    }
}

#[test]
fn observations_alone_cannot_mint_a_checked_forwarder() {
    let facts = original(0);
    let observation = serde_json::json!({});
    let from = ReferenceObservation {
        canonical: &facts,
        observation: &observation,
    };
    let new = ReferenceObservation {
        canonical: &facts,
        observation: &observation,
    };
    assert!(verify(from, new, Haystack::Bytes).is_err());
}
