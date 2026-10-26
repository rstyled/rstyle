//! Diagnostic observations and concrete-looking raw facts cannot select a helper.
use super::*;
use crate::{HirFnFacts, data_origins::Origin, field_locations::Projection};
use serde_json::json;

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../reference_instances/forwarder/fixtures.json"
    ))
    .unwrap()
}

fn assert_raw_rejected(facts: &HirFnFacts) {
    let call = &facts.body.children[0];
    assert!(call.res.is_none());
    assert!(call.dispatch.is_none());
    assert!(super::resolution(call, facts).is_none());
    assert!(crate::helper_calls::selected_resolution(call, facts).is_none());
    assert!(crate::helper_calls::callee(call, facts).is_none());
    assert!(!super::verify(call, facts, facts).unwrap());
    assert!(!crate::reference_instances::forwarder_activation::verify(facts).unwrap());
    assert!(crate::helper_calls::verify_use(call, facts).is_err());
}

#[test]
fn original_slice_and_string_from_observations_grant_no_selected_call_authority() {
    for observation in fixtures() {
        let facts: HirFnFacts = serde_json::from_value(observation["original"].clone()).unwrap();
        assert_raw_rejected(&facts);
        let round_trip: HirFnFacts =
            serde_json::from_slice(&serde_json::to_vec(&facts).unwrap()).unwrap();
        assert_raw_rejected(&round_trip);
    }
}

#[test]
fn normalized_formals_and_serialized_activation_claims_cannot_mint_authority() {
    for observation in fixtures() {
        let mut facts: HirFnFacts =
            serde_json::from_value(observation["original"].clone()).unwrap();
        let token = crate::reference_instances::shared_bytes().text().unwrap();
        facts.signature.params[0].ty.normalized = Some(token.clone());
        facts.signature.params[0]
            .pat
            .ty
            .as_mut()
            .unwrap()
            .normalized = Some(token.clone());
        facts.bindings[0].ty.normalized = Some(token.clone());
        facts.body.children[0].children[0]
            .ty
            .as_mut()
            .unwrap()
            .normalized = Some(token);
        facts.build.frontend_options.insert(
            crate::reference_instances::forwarder_activation::CAPABILITY.into(),
            json!({"checked": true, "observation": observation}),
        );
        assert_raw_rejected(&facts);
    }
}

#[test]
fn historical_profiles_cannot_turn_a_raw_from_call_into_a_selected_dependency() {
    for profile in [
        "r0",
        "hir-helpers-v1",
        "hir-data-v1",
        "hir-data-v2",
        "hir-data-v3",
    ] {
        let mut facts: HirFnFacts =
            serde_json::from_value(fixtures()[0]["original"].clone()).unwrap();
        facts.build.frontend_options.insert(
            crate::extraction_profile::SELECTION_KEY.into(),
            json!(profile),
        );
        assert_raw_rejected(&facts);
    }
}

#[test]
fn raw_string_formals_do_not_authorize_a_utf8_returned_component() {
    let mut facts: HirFnFacts = serde_json::from_value(fixtures()[1]["original"].clone()).unwrap();
    facts.signature.params[0].ty.normalized =
        Some(r#"{"kind":"shared","value":{"kind":"str"}}"#.into());
    let origin = Origin {
        parameter: 0,
        path: vec![Projection::SharedReferent, Projection::Utf8Bytes],
    };
    assert!(
        crate::reference_instances::forwarder_activation::verify_returned_origin(
            &origin,
            &facts,
            &crate::reference_instances::shared_bytes(),
        )
        .is_err()
    );
}
