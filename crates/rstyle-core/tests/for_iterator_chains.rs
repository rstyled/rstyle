//! `for` consumes the existing contract-modeled adapter pipeline, including callbacks.
//! Captured by the pinned compiler from `src/cpg/testdata/for-iterator-chains.rs`.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::runtime_bounds::{self, Assessment, Bound, Outcome, Program};
use serde_json::{Value, json};

fn document() -> Value {
    serde_json::from_str(include_str!(
        "../src/cpg/testdata/for-iterator-chains.cpg.json"
    ))
    .unwrap()
}

fn program(document: Value, name: &str) -> Program {
    let programs = rstyle_core::cpg::run_bounds(vec![document]).unwrap();
    let (_, input) = programs
        .into_iter()
        .find(|(identity, _)| identity.ends_with(&format!("::{name}")))
        .unwrap();
    let Input::Captured { program } = input else {
        panic!("expected a captured program: {input:?}");
    };
    program
}

fn proven(name: &str) -> Assessment {
    let result = runtime_bounds::analyze(&program(document(), name));
    let Outcome::Proven(assessment) = result else {
        panic!("expected a proven loop bound for {name}: {result:?}");
    };
    assert!(
        !assessment.loops.is_empty(),
        "a source scan must be modeled"
    );
    assessment
}

fn unknown(document: Value, name: &str) {
    let result = runtime_bounds::analyze(&program(document, name));
    assert!(
        matches!(result, Outcome::Unknown { .. }),
        "{name}: {result:?}"
    );
}

fn function<'a>(document: &'a mut Value, name: &str) -> &'a mut Value {
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|function| function["name"] == name)
        .unwrap()
}

fn pattern<'a>(document: &'a mut Value, name: &str) -> &'a mut Value {
    let function = function(document, name);
    let nodes = function["nodes"].as_array_mut().unwrap();
    let id = nodes.iter().find(|node| node["kind"] == "for").unwrap()["children"][0].clone();
    nodes.iter_mut().find(|node| node["id"] == id).unwrap()
}

fn closure<'a>(document: &'a mut Value, name: &str) -> &'a mut Value {
    function(document, name)["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["kind"] == "closure")
        .unwrap()
}

#[test]
fn shared_sources_keep_input_dependent_extents_through_pass_adapters() {
    for name in ["enumerate", "reverse"] {
        let assessment = proven(name);
        assert_eq!(assessment.loops.len(), 1);
        assert_eq!(assessment.loop_body_entries.evaluate(&|_| Some(7)), Some(7));
        assert!(matches!(assessment.loop_body_entries, Bound::Input(_)));
    }
}

#[test]
fn a_fixed_extent_stays_concrete_through_a_pass_adapter() {
    assert_eq!(proven("fixed").loop_body_entries, Bound::Const(4));
}

#[test]
fn structurally_safe_fixed_array_yields_need_no_implicit_cleanup_call() {
    assert_eq!(proven("fixed_yield").loop_body_entries, Bound::Const(4));
}

#[test]
fn a_filter_counts_its_callback_and_the_for_body_per_source_item() {
    // Four pulls + the predicate's three entries per pull + two body entries per yield.
    let assessment = proven("callback_cost");
    assert_eq!(assessment.loops.len(), 3);
    assert_eq!(assessment.loop_body_entries.evaluate(&|_| None), Some(24));
}

#[test]
fn for_body_early_exits_preserve_the_source_scan_bound() {
    for name in ["early_break", "early_continue", "early_return"] {
        let assessment = proven(name);
        assert_eq!(assessment.loops.len(), 2);
        assert_eq!(assessment.loop_body_entries.evaluate(&|_| None), Some(12));
    }
}

#[test]
fn returning_from_an_adapter_callback_still_reaches_the_for_body() {
    assert_eq!(
        proven("callback_return")
            .loop_body_entries
            .evaluate(&|_| None),
        Some(24)
    );
    assert_eq!(
        proven("callback_unconditional_return")
            .loop_body_entries
            .evaluate(&|_| None),
        Some(12)
    );
}

#[test]
fn adapter_arguments_are_evaluated_once_before_the_scan() {
    let assessment = proven("evaluated_once");
    assert_eq!(assessment.calls.len(), 1);
    assert!(assessment.calls[0].callee.contains("argument"));
    assert_eq!(assessment.calls[0].count, Bound::Const(1));
    assert_eq!(assessment.loop_body_entries.evaluate(&|_| Some(7)), Some(7));
}

#[test]
fn mutable_custom_and_unresolved_traversals_stay_unknown() {
    for name in [
        "mutable_source",
        "unknown_callback",
        "unknown_source",
        "unmodeled_flat_map",
        "growing_source",
        "custom_clone",
    ] {
        unknown(document(), name);
    }
}

#[test]
fn named_callbacks_do_not_hide_cleanup_of_owned_or_nested_yields() {
    for name in ["owned_drop", "nested_drop", "tuple_drop", "ref_drop"] {
        unknown(document(), name);
    }
}

#[test]
fn every_intermediate_adapter_item_must_have_known_safe_cleanup() {
    unknown(document(), "intermediate_drop");
    let mut document = document();
    let references = function(&mut document, "reverse")["refs"]
        .as_array_mut()
        .unwrap();
    for reference in references {
        reference.as_object_mut().unwrap().remove("item");
    }
    unknown(document, "reverse");
}

#[test]
fn dependent_adapter_references_do_not_certify_item_cleanup() {
    let mut document = document();
    let reference = function(&mut document, "reverse")["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference.get("item").is_some())
        .unwrap();
    reference["status"] = json!("dependent");
    reference["via"] = reference["target"]["id"].clone();
    reference.as_object_mut().unwrap().remove("target");
    unknown(document, "reverse");
}

#[test]
fn owning_callback_captures_do_not_disappear_with_adapter_construction() {
    unknown(document(), "captured_guard");
    let mut document = document();
    document["coverage"]["capabilities"]["implicit-calls"]["status"] = json!("supported");
    unknown(document, "captured_guard");
}

#[test]
fn borrowed_guard_captures_do_not_own_the_guard() {
    for name in ["shared_capture", "moved_reference_capture"] {
        let assessment = proven(name);
        assert_eq!(assessment.loop_body_entries, Bound::Const(4));
        assert_eq!(assessment.calls.len(), 1);
        assert_eq!(assessment.calls[0].count, Bound::Const(4));
    }
}

#[test]
fn named_primitive_callbacks_retain_invocations_and_their_own_loop_work() {
    let assessment = proven("named_map");
    assert_eq!(assessment.loop_body_entries, Bound::Const(4));
    assert_eq!(assessment.calls.len(), 1);
    assert!(assessment.calls[0].callee.contains("named_item"));
    assert_eq!(assessment.calls[0].count, Bound::Const(4));

    let demand = rstyle_core::cpg::records::Demand {
        bounds: true,
        ..Default::default()
    };
    let records = rstyle_core::cpg::run_records(vec![document()], demand).unwrap();
    let report = serde_json::to_value(rstyle_core::cpg::bounds::report::from_records(
        &records, None,
    ))
    .unwrap();
    let row = report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "named_map")
        .unwrap();
    // Four source pulls plus three callback-loop entries for each invocation.
    assert_eq!(
        row["composed"]["Proven"]["loop_body_entries"],
        json!({"Const": 16})
    );
    assert_eq!(
        row["composed"]["Proven"]["workspace_invocations"],
        json!({"Const": 4})
    );
}

#[test]
fn opaque_and_owning_pointer_yields_need_explicit_cleanup_safety() {
    for ty in [
        json!({"id": "test:unknown-yield", "kind": "opaque", "spelling": "unknown"}),
        json!({"id": "test:unknown-yield", "kind": "pointer", "mode": "owned", "spelling": "owned unknown"}),
    ] {
        let mut document = document();
        document["unit"]["types"].as_array_mut().unwrap().push(ty);
        pattern(&mut document, "fixed_yield")["type"] = json!("test:unknown-yield");
        unknown(document, "fixed_yield");
    }
}

#[test]
fn missing_capture_rosters_and_unmodeled_capture_modes_stay_unknown() {
    let mut missing = document();
    closure(&mut missing, "callback_cost")
        .as_object_mut()
        .unwrap()
        .remove("captures");
    unknown(missing, "callback_cost");

    let mut unmodeled = document();
    closure(&mut unmodeled, "shared_capture")["captures"][0]["mode"] = json!("use");
    unknown(unmodeled, "shared_capture");
}

#[test]
fn capture_rosters_share_the_total_cleanup_work_budget() {
    let mut document = document();
    let callback = closure(&mut document, "shared_capture");
    callback["captures"] = json!(vec![callback["captures"][0].clone(); 300]);
    unknown(document, "shared_capture");
}

#[test]
fn terminal_consumers_keep_their_existing_lowering() {
    let mut document = document();
    closure(&mut document, "terminal")
        .as_object_mut()
        .unwrap()
        .remove("captures");
    let result = runtime_bounds::analyze(&program(document, "terminal"));
    let Outcome::Proven(assessment) = result else {
        panic!("existing terminal consumer behavior changed: {result:?}");
    };
    assert_eq!(assessment.loop_body_entries, Bound::Const(4));
}

#[test]
fn ignored_binding_cleanup_stays_unknown_even_when_other_implicit_calls_are_emitted() {
    let mut document = document();
    document["coverage"]["capabilities"]["implicit-calls"]["status"] = json!("supported");
    unknown(document, "nested_drop");
}

#[test]
fn missing_yield_type_facts_do_not_default_to_trivial_cleanup() {
    let mut document = document();
    pattern(&mut document, "reverse")
        .as_object_mut()
        .unwrap()
        .remove("type");
    unknown(document, "reverse");
}

#[test]
fn nominal_and_generic_metadata_prevent_trivial_cleanup_assumptions() {
    for kind in ["string", "sequence", "tuple", "integer"] {
        for nominal in [true, false] {
            let mut document = document();
            let mut ty = json!({"id": "test:yield", "kind": kind, "spelling": "yield"});
            match kind {
                "sequence" => {
                    ty["fixed"] = json!(true);
                    ty["element"] = json!("type:u8");
                }
                "tuple" => ty["items"] = json!(["type:u8"]),
                "integer" => {
                    ty["bits"] = json!(8);
                    ty["signed"] = json!(false);
                }
                _ => {}
            }
            if nominal {
                ty["definition"] = document["unit"]["types"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|ty| ty["id"] == "type:Guard")
                    .unwrap()["definition"]
                    .clone();
            } else {
                ty["arguments"] = json!(["type:u8"]);
            }
            document["unit"]["types"].as_array_mut().unwrap().push(ty);
            pattern(&mut document, "fixed_yield")["type"] = json!("test:yield");
            unknown(document, "fixed_yield");
        }
    }
}

#[test]
fn recursive_yield_type_facts_exhaust_a_guard_instead_of_recursing_forever() {
    let mut document = document();
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "test:cycle", "kind": "tuple", "spelling": "cycle", "items": ["test:cycle"]
        }));
    pattern(&mut document, "reverse")["type"] = json!("test:cycle");
    unknown(document, "reverse");
}

#[test]
fn shared_yield_type_dags_have_one_total_work_budget() {
    let mut document = document();
    let types = document["unit"]["types"].as_array_mut().unwrap();
    let mut previous = "type:u8".to_string();
    for index in 0..10 {
        let id = format!("test:tuple-{index}");
        types.push(
            json!({"id": id, "kind": "tuple", "spelling": "pair", "items": [previous, previous]}),
        );
        previous = id;
    }
    pattern(&mut document, "reverse")["type"] = json!(previous);
    unknown(document, "reverse");
}

#[test]
fn unresolved_adapter_dispatch_does_not_inherit_a_standard_pipeline() {
    let mut document = document();
    let function = function(&mut document, "reverse");
    let adapter = function["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| {
            reference["target"]["id"] == "def:core::iter::traits::iterator::Iterator::rev"
        })
        .unwrap();
    adapter["status"] = json!("unresolved");
    adapter.as_object_mut().unwrap().remove("target");
    unknown(document, "reverse");
}

#[test]
fn a_for_target_that_reuses_a_source_place_is_not_a_new_binding() {
    let mut document = document();
    let function = function(&mut document, "reverse");
    let source = function["signature"]["parameters"][0]["symbol"].clone();
    let nodes = function["nodes"].as_array_mut().unwrap();
    let pattern = nodes
        .iter_mut()
        .find(|node| node["kind"] == "pattern_binding")
        .unwrap();
    pattern["kind"] = json!("identifier");
    let id = pattern["id"].clone();
    function["refs"].as_array_mut().unwrap().push(json!({
        "node": id, "status": "resolved", "target": {"kind": "symbol", "id": source}
    }));
    unknown(document, "reverse");
}
