//! Fixed sequence extents use compiler facts, including Rust's shared array-to-slice
//! adjustment. The capture comes from `src/cpg/testdata/fixed-sequences.rs`.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::runtime_bounds::{self, Action, Bound, Outcome, Program};
use serde_json::{Value, json};

fn document() -> Value {
    serde_json::from_str(include_str!("../src/cpg/testdata/fixed-sequences.cpg.json")).unwrap()
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

fn bound(document: Value, name: &str) -> Bound {
    let result = runtime_bounds::analyze(&program(document, name));
    let Outcome::Proven(assessment) = result else {
        panic!("expected a proven loop bound: {result:?}");
    };
    assert!(!assessment.loops.is_empty(), "a real scan must be modeled");
    assessment.loop_body_entries
}

fn array(document: &mut Value) -> &mut Value {
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["id"] == "type:[u8; 4]")
        .unwrap()
}

fn iter_receiver(document: &mut Value) -> &mut Value {
    let function = document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|function| function["name"] == "iter")
        .unwrap();
    function["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["kind"] == "identifier" && node["adjustments"].is_array())
        .unwrap()
}

fn unknown(document: Value, name: &str) {
    let result = runtime_bounds::analyze(&program(document, name));
    assert!(matches!(result, Outcome::Unknown { .. }), "{result:?}");
}

#[test]
fn fixed_arrays_bound_direct_shared_scans_and_resolved_traversals() {
    for name in ["borrowed", "iter", "chain", "indexed"] {
        assert_eq!(bound(document(), name), Bound::Const(4), "{name}");
    }
}

#[test]
fn fixed_extent_chains_count_each_callbacks_own_loops() {
    // Four source pulls and the callback's three loop entries for each pull.
    assert_eq!(
        bound(document(), "callback_loops").evaluate(&|_| None),
        Some(16)
    );
}

#[test]
fn a_zero_length_array_keeps_the_scan_with_zero_entries() {
    let mut document = document();
    array(&mut document)["length"] = json!(0);
    assert_eq!(bound(document, "borrowed"), Bound::Const(0));
}

#[test]
fn fixed_sequences_without_a_compiler_known_length_stay_unknown() {
    unknown(document(), "const_generic");
}

#[test]
fn a_fixed_source_does_not_resolve_a_handed_function_pointer() {
    unknown(document(), "handed_callback");
}

#[test]
fn owned_and_writable_arrays_do_not_gain_a_borrowed_traversal_model() {
    unknown(document(), "writable");
    // An owned array is a sequence of one fixed length, which a `for` counts by that length.
    assert_eq!(bound(document(), "owned"), Bound::Const(4));
}

#[test]
fn a_length_without_fixed_sequence_semantics_is_no_constant() {
    let mut document = document();
    array(&mut document)["fixed"] = json!(false);
    assert!(matches!(bound(document, "borrowed"), Bound::Input(_)));
}

#[test]
fn an_extent_larger_than_the_target_counter_stays_unknown() {
    let mut document = document();
    document["unit"]["target"]["pointer_width"] = json!(32);
    array(&mut document)["length"] = json!(1u64 << 32);
    unknown(document, "borrowed");
}

#[test]
fn a_fixed_length_fitting_a_narrower_target_is_preserved() {
    let mut document = document();
    document["unit"]["target"]["pointer_width"] = json!(32);
    assert_eq!(bound(document, "borrowed"), Bound::Const(4));
}

#[test]
fn a_nonstandard_iter_method_does_not_inherit_the_extent_contract() {
    let mut document = document();
    for definition in document["unit"]["definitions"].as_array_mut().unwrap() {
        if definition["operation"] == "rust:slice.iter" {
            definition["standard"] = json!(false);
        }
    }
    unknown(document, "iter");
}

#[test]
fn a_length_of_a_computed_array_does_not_erase_its_receiver_call() {
    let program = program(document(), "computed_read");
    assert!(program.nodes.iter().any(|node| {
        matches!(&node.action, Action::Call { callee, .. } if callee.contains("computed_receiver"))
    }));
    unknown(document(), "computed_length");
}

#[test]
fn an_unrecognized_array_adjustment_stays_unknown() {
    let mut document = document();
    let adjustments = iter_receiver(&mut document)["adjustments"]
        .as_array_mut()
        .unwrap();
    adjustments.last_mut().unwrap()["kind"] = json!("overloaded_deref");
    unknown(document, "iter");
}

#[test]
fn unsizing_requires_shared_references_and_complete_array_to_slice_types() {
    let types = [
        (None, Some("type:&[u8]")),
        (Some("type:&[u8; 4]"), None),
        (Some("type:[u8; 4]"), Some("type:&[u8]")),
        (Some("type:&[u8; 4]"), Some("type:[u8]")),
        (Some("type:&[u8; 4]"), Some("type:&[u8; 4]")),
    ];
    for (source, target) in types {
        let mut document = document();
        let receiver = iter_receiver(&mut document);
        receiver["type"] = json!(source);
        receiver["adjustments"] = json!([{ "kind": "unsize", "target": target }]);
        receiver
            .as_object_mut()
            .unwrap()
            .retain(|_, value| !value.is_null());
        receiver["adjustments"][0]
            .as_object_mut()
            .unwrap()
            .retain(|_, value| !value.is_null());
        unknown(document, "iter");
    }
}

#[test]
fn an_array_to_slice_coercion_cannot_change_element_types() {
    let mut document = document();
    let slice = document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["id"] == "type:[u8]")
        .unwrap();
    slice["element"] = json!("type:usize");
    unknown(document, "iter");
}

#[test]
fn an_incomplete_referent_does_not_prove_an_array_to_slice_coercion() {
    for id in ["type:&[u8; 4]", "type:&[u8]"] {
        let mut document = document();
        let reference = document["unit"]["types"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|ty| ty["id"] == id)
            .unwrap();
        reference.as_object_mut().unwrap().remove("target");
        unknown(document, "iter");
    }
}
