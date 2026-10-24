//! Original-fact preservation and independent rejection of extra value uses.
use super::*;
use crate::product_instances::test_support::conversion as fixture;

#[test]
fn concrete_projection_changes_only_four_unresolved_type_slots() {
    let (canonical, _, span) = fixture();
    let formal = &canonical.signature.params[1].pat.hir;
    let token = crate::data_types::token(&span).unwrap();
    let mut projected = concrete_facts(&canonical, formal, &token).unwrap();
    assert_eq!(
        projected.signature.params[1].ty.normalized.as_deref(),
        Some(token.as_str())
    );
    assert_eq!(
        projected.signature.params[1]
            .pat
            .ty
            .as_ref()
            .unwrap()
            .normalized
            .as_deref(),
        Some(token.as_str())
    );
    let binding = projected
        .bindings
        .iter_mut()
        .find(|binding| binding.hir == *formal)
        .unwrap();
    assert_eq!(binding.ty.normalized.as_deref(), Some(token.as_str()));
    binding.ty.normalized = None;
    let read = &mut projected.body.children[0].children[2].children[0];
    assert_eq!(
        read.ty.as_ref().unwrap().normalized.as_deref(),
        Some(token.as_str())
    );
    read.ty.as_mut().unwrap().normalized = None;
    projected.signature.params[1].ty.normalized = None;
    projected.signature.params[1]
        .pat
        .ty
        .as_mut()
        .unwrap()
        .normalized = None;
    assert_eq!(projected, canonical);
}

#[test]
fn extra_reads_bindings_or_already_instantiated_slots_fail_closed() {
    let (canonical, _, span) = fixture();
    let token = crate::data_types::token(&span).unwrap();
    let formal = canonical.signature.params[1].pat.hir.clone();
    let mut duplicate = canonical.clone();
    duplicate
        .body
        .children
        .push(duplicate.body.children[0].children[2].children[0].clone());
    assert!(concrete_facts(&duplicate, &formal, &token).is_err());
    let mut duplicate = canonical.clone();
    duplicate.bindings.push(
        duplicate
            .bindings
            .iter()
            .find(|binding| binding.hir == formal)
            .unwrap()
            .clone(),
    );
    assert!(concrete_facts(&duplicate, &formal, &token).is_err());
    let mut changed = canonical.clone();
    changed.signature.params[1].ty.normalized = Some(token.clone());
    assert!(concrete_facts(&changed, &formal, &token).is_err());
    changed.signature.params[1].ty.normalized = None;
    changed.signature.params[1].ty.observed = "a different generic".into();
    assert!(concrete_facts(&changed, &formal, &token).is_err());
    assert!(verify(&canonical, &Value::Null, SpanArgument::Span).is_err());
}

#[test]
fn retained_operation_contains_only_the_proved_post_normalization_change() {
    let (canonical, _, span) = fixture();
    let formal = &canonical.signature.params[1].pat.hir;
    let token = crate::data_types::token(&span).unwrap();
    let original = &canonical.body.children[0].children[2];
    let mut projected = concrete_facts(&canonical, formal, &token).unwrap();
    let retained = concrete_operation(&projected, original, formal, &token).unwrap();
    assert_eq!(retained, projected.body.children[0].children[2]);
    assert_ne!(retained, *original);
    assert_eq!(
        retained.children[0].ty.as_ref().unwrap().normalized,
        Some(token.clone())
    );
    projected.body.children[0].children[2].snippet.push(' ');
    assert!(concrete_operation(&projected, original, formal, &token).is_err());
    projected.body.children[0].children[2] = retained.clone();
    projected.body.children.push(retained);
    assert!(concrete_operation(&projected, original, formal, &token).is_err());
}

#[test]
fn original_receiver_inventory_remains_required_before_concrete_substitution() {
    let (mut canonical, _, _) = fixture();
    canonical.build.frontend_options.insert(
        crate::data_receivers::KEY.into(),
        serde_json::from_str(include_str!("receiver.json")).unwrap(),
    );
    let call = &canonical.body.children[0].children[2];
    verify_original_receiver(&canonical, call).unwrap();
    for mode in [
        "missing",
        "duplicate",
        "receiver",
        "result",
        "input",
        "adjustment",
    ] {
        let mut changed = canonical.clone();
        let mut capture = changed
            .build
            .frontend_options
            .data::<crate::data_receivers::Capture>()
            .unwrap()
            .into_owned();
        match mode {
            "missing" => capture.methods.clear(),
            "duplicate" => capture.methods.push(capture.methods[0].clone()),
            "receiver" => capture.methods[0].receiver.local += 1,
            "input" => capture.methods[0].signature_inputs.clear(),
            "result" => {
                capture.methods[0].signature_result = Observation::Unsupported {
                    reason: "missing result evidence".into(),
                }
            }
            "adjustment" => {
                let target = capture.methods[0].receiver_type.clone();
                capture.methods[0]
                    .adjustments
                    .push(crate::data_receivers::Adjustment {
                        kind: crate::data_receivers::Kind::SharedBorrow,
                        target,
                    })
            }
            _ => unreachable!(),
        }
        changed.build.frontend_options.insert(
            crate::data_receivers::KEY.into(),
            serde_json::to_value(capture).unwrap(),
        );
        assert!(verify_original_receiver(&changed, call).is_err(), "{mode}");
    }
}
