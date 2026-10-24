//! Range normalization retains original source and raw generic receiver evidence.
use super::*;
use crate::product_instances::test_support::conversion as fixture;

#[test]
fn range_projection_only_replaces_the_proved_generic_formal_and_read() {
    let (mut canonical, _, _) = fixture();
    canonical.build.frontend_options.insert(
        crate::data_receivers::KEY.into(),
        serde_json::from_str(include_str!("../receiver.json")).unwrap(),
    );
    let raw = canonical.build.frontend_options.clone();
    let formal = canonical.signature.params[1].pat.hir.clone();
    let projected = concrete_facts(&canonical, &formal, crate::range_values::TOKEN).unwrap();
    assert_eq!(projected.source, canonical.source);
    assert_eq!(projected.fn_span, canonical.fn_span);
    assert_eq!(projected.source_diagnostics, canonical.source_diagnostics);
    assert_eq!(
        projected.source_scan_complete,
        canonical.source_scan_complete
    );
    assert_eq!(projected.build.frontend_options, raw);
    assert_eq!(projected.signature.params[0], canonical.signature.params[0]);
    let operation = concrete_operation(
        &projected,
        &canonical.body.children[0].children[2],
        &formal,
        crate::range_values::TOKEN,
    )
    .unwrap();
    assert_eq!(
        operation.children[0].res,
        canonical.body.children[0].children[2].children[0].res
    );
    assert_eq!(
        operation.children[0]
            .ty
            .as_ref()
            .unwrap()
            .normalized
            .as_deref(),
        Some(crate::range_values::TOKEN)
    );
    verify_original_receiver(&canonical, &canonical.body.children[0].children[2]).unwrap();
}

#[test]
fn range_without_actual_enriched_local_callee_still_has_no_projection() {
    let (canonical, observation, _) = fixture();
    assert!(verify(&canonical, &observation, SpanArgument::RangeUsize).is_err());
    assert!(verify_range_constructor_projection(&canonical, &observation, &canonical).is_err());
    let mut changed = canonical.clone();
    changed
        .body
        .children
        .push(changed.body.children[0].children[2].children[0].clone());
    let formal = &canonical.signature.params[1].pat.hir;
    assert!(concrete_facts(&changed, formal, crate::range_values::TOKEN).is_err());
}
