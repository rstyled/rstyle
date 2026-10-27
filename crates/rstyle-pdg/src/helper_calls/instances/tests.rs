//! Association validation controls; they do not manufacture a checked projection.
use super::*;
use serde_json::json;

fn fixture() -> (crate::HirFnFacts, HirNode, Value) {
    let fixtures: Value = serde_json::from_str(include_str!(
        "../../scalar_instances/provenance/fixtures.json"
    ))
    .unwrap();
    let mut facts: crate::HirFnFacts =
        serde_json::from_value(fixtures[0]["facts"].clone()).unwrap();
    facts.signature.return_ty.normalized = Some("i32".into());
    facts.signature.return_ty.spelling = "T".into();
    let owner = facts.build.frontend_options.get(COMPILED).unwrap();
    let path = format!("{}::{}", owner["crate_name"].as_str().unwrap(), facts.name);
    let definition = json!({"path":path,"definition_identity":facts.build.frontend_options.get("definition_identity").unwrap(),
        "crate":owner["crate_name"],"stable_crate_id":owner["stable_crate_id"],"local":false});
    let argument = json!({"kind":"type","value":"untrusted diagnostic text","display_only":true,
        "identity_status":"known","identity":{"kind":"builtin","name":"i32"}});
    let mut compiled = owner.clone();
    compiled["rmeta"] = json!("/selected/dependency.rmeta");
    compiled["crate_hash"] = json!("captured-native-crate-hash");
    compiled["instance"] = json!({"type_arguments":[],"const_arguments":[]});
    let mut node = facts.signature.params[0].pat.clone();
    node.kind = "expr_call".into();
    node.expansion = None;
    node.children = facts
        .signature
        .params
        .iter()
        .map(|param| param.pat.clone())
        .collect();
    node.dispatch = Some(crate::Dispatch {
        kind: "resolved_scalar_instance".into(),
        op: path,
        operand_tys: vec!["i32".into(); node.children.len()],
        result_ty: "i32".into(),
    });
    let observation = json!({"hir":node.hir,"span":node.span,"expanded":false,"requested":definition,
        "requested_arguments":[argument],"compiled_dependency":compiled,"status":"resolved",
        "selected":definition,"selected_arguments":[argument],
        "signature":{"display_only":true,"inputs":node.dispatch.as_ref().unwrap().operand_tys,"output":"i32"}});
    (facts, node, observation)
}

fn check(facts: &crate::HirFnFacts, node: &HirNode, observation: Value) -> Result<(), String> {
    let observation: Observation =
        serde_json::from_value(observation).map_err(|e| e.to_string())?;
    validate(
        node,
        &observation,
        facts,
        scalar_instances::ScalarArgument::I32,
    )
}

#[test]
fn complete_structural_call_contract_accepts_but_raw_facts_have_no_instance_authority() {
    let (facts, node, observation) = fixture();
    check(&facts, &node, observation.clone()).unwrap();
    assert!(signature(&node, &facts, &facts).is_err());
    assert_eq!(facts.signature.generics[0].name, "T");
}

#[test]
fn site_definition_build_and_substitution_transplants_are_rejected() {
    let (facts, node, observation) = fixture();
    for pointer in ["/hir/local", "/span/start"] {
        let mut changed = observation.clone();
        *changed.pointer_mut(pointer).unwrap() = json!(999999);
        assert!(check(&facts, &node, changed).is_err(), "{pointer}");
    }
    for pointer in [
        "/requested/definition_identity",
        "/selected/stable_crate_id",
        "/compiled_dependency/stable_crate_id",
        "/requested_arguments/0/identity/name",
        "/selected_arguments/0/identity/name",
    ] {
        let mut changed = observation.clone();
        *changed.pointer_mut(pointer).unwrap() = json!("wrong");
        assert!(check(&facts, &node, changed).is_err(), "{pointer}");
    }
    for field in ["requested_arguments", "selected_arguments"] {
        let mut changed = observation.clone();
        changed[field] = json!([]);
        assert!(check(&facts, &node, changed).is_err());
    }
    let mut changed = observation;
    changed["expanded"] = json!(true);
    assert!(check(&facts, &node, changed).is_err());
}

#[test]
fn concrete_operand_and_result_types_are_required_independently_of_display_signature() {
    let (facts, node, observation) = fixture();
    let mut changed = node.clone();
    changed.children[0].ty.as_mut().unwrap().normalized = Some("u32".into());
    assert!(check(&facts, &changed, observation.clone()).is_err());
    changed = node.clone();
    changed.ty.as_mut().unwrap().normalized = Some("u32".into());
    assert!(check(&facts, &changed, observation.clone()).is_err());
    changed = node;
    changed.dispatch.as_mut().unwrap().kind = "resolved_direct".into();
    assert!(check(&facts, &changed, observation).is_err());
}

#[test]
fn call_inventory_is_complete_unique_and_exact() {
    let (mut caller, node, observation) = fixture();
    let capture = json!({"identity":"rstyle.rust.call-instance-observations/1","complete":true,"calls":[observation]});
    caller
        .build
        .frontend_options
        .insert(KEY.into(), capture.clone());
    assert_eq!(call_observation(&node, &caller).unwrap(), &observation);
    assert_eq!(
        call_argument(&node, &caller).unwrap(),
        scalar_instances::ScalarArgument::I32
    );
    for calls in [json!([]), json!([observation, observation])] {
        let mut changed = capture.clone();
        changed["calls"] = calls;
        caller.build.frontend_options.insert(KEY.into(), changed);
        assert!(call_observation(&node, &caller).is_err());
    }
    let mut changed = capture;
    changed["complete"] = json!(false);
    caller.build.frontend_options.insert(KEY.into(), changed);
    assert!(call_observation(&node, &caller).is_err());
}

#[test]
fn requested_and_selected_scalar_arguments_must_agree_before_linking() {
    let (mut caller, node, observation) = fixture();
    for pointer in [
        "/selected_arguments/0/identity/name",
        "/requested_arguments/0/identity/name",
    ] {
        let mut changed = observation.clone();
        *changed.pointer_mut(pointer).unwrap() = json!("u32");
        caller.build.frontend_options.insert(KEY.into(), json!({
            "identity":"rstyle.rust.call-instance-observations/1","complete":true,"calls":[changed]}));
        assert!(call_argument(&node, &caller).is_err());
    }
}
