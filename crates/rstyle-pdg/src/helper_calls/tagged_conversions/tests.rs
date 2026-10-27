//! Original native contract records, never fabricated checked source receipts.
use super::*;
use crate::copy_tagged_values::origins::{CaseKind, Component, Condition, Contract};
use serde_json::{Value, json};

fn original(direction: &str) -> WireContract {
    let values: Value = serde_json::from_str(include_str!("tests/original-outputs.json")).unwrap();
    serde_json::from_value(values[direction].clone()).unwrap()
}

fn facts(inverse: bool) -> crate::HirFnFacts {
    serde_json::from_str(if inverse {
        include_str!("../../copy_tagged_values/tests/1.json")
    } else {
        include_str!("../../copy_tagged_values/tests/0.json")
    })
    .unwrap()
}

fn original_summary(inverse: bool) -> super::super::Summary {
    summary(
        &facts(inverse),
        original(if inverse { "inverse" } else { "wrap" }),
    )
    .unwrap()
}

#[test]
fn native_contract_summaries_preserve_nominal_routes_and_explicit_empty_error() {
    let wrapped = original_summary(false);
    let output = validate_shape(&wrapped).unwrap();
    assert_eq!(output.input_cases[0].kind, CaseKind::Whole);
    assert_eq!(output.alternatives.len(), 1);
    assert!(matches!(
        output.alternatives[0].condition,
        Condition::Always
    ));
    assert!(matches!(
        output.alternatives[0].leaves[0].output[0],
        Component::EnumPayload { .. }
    ));

    let inverse = original_summary(true);
    let output = validate_shape(&inverse).unwrap();
    assert_eq!(output.input_cases.len(), 2);
    assert_eq!(output.output_cases.len(), 2);
    assert_eq!(output.output_cases[1].kind, CaseKind::ResultErr);
    assert!(output.output_cases[1].components.is_empty());
    assert_eq!(output.alternatives[0].output, CaseKind::ResultErr);
    assert!(output.alternatives[0].leaves.is_empty());
    assert!(matches!(
        output.alternatives[0].condition,
        Condition::Complement(_)
    ));
    assert!(matches!(
        output.alternatives[1].condition,
        Condition::Selected(_)
    ));

    for expected in [wrapped, inverse] {
        let wire = serde_json::to_value(&expected).unwrap();
        assert!(wire.get("reference_outputs").is_none());
        assert!(wire.get("component_outputs").is_none());
        let actual: super::super::Summary = serde_json::from_value(wire).unwrap();
        assert!(actual == expected);
    }
}

#[test]
fn summary_shape_rejects_missing_outputs_effects_and_mixed_legacy_origins() {
    for inverse in [false, true] {
        let expected = serde_json::to_value(original_summary(inverse)).unwrap();
        for (field, value) in [
            ("identity", json!("rstyle.helper.source-body/1")),
            ("result_dependence", json!("flattened-origins")),
            ("argument_evaluation", json!("unordered")),
            ("argument_types", json!([])),
            ("result_type", json!("")),
            ("mutates_arguments", json!(true)),
            ("memory_effects", json!(true)),
            ("may_panic", json!(true)),
            ("always_returns", json!(false)),
            ("reference_outputs", json!([])),
            ("component_outputs", json!([])),
            ("memory_reads", json!([])),
            (
                "bounded_loop",
                json!({"may_diverge":false,"positive_max_backedges":1}),
            ),
            ("tagged_output", Value::Null),
        ] {
            let mut changed = expected.clone();
            changed[field] = value;
            let changed: super::super::Summary = serde_json::from_value(changed).unwrap();
            assert!(validate_shape(&changed).is_err(), "{field}");
        }
    }
}

#[test]
fn historical_summary_wire_omits_the_new_field_without_inventing_output() {
    let mut original = original_summary(false);
    original.identity = "rstyle.helper.source-body/1".into();
    original.result_dependence = "call-input-snapshots-with-verified-callee-return-graph/1".into();
    original.tagged_output = None;
    let wire = serde_json::to_value(&original).unwrap();
    assert!(wire.get("tagged_output").is_none());
    let decoded: super::super::Summary = serde_json::from_value(wire.clone()).unwrap();
    assert!(decoded == original);
    assert!(decoded.tagged_output.is_none());
    assert!(validate_shape(&decoded).is_err());

    let mut unknown = wire;
    unknown["unverified_tagged_output"] = json!({});
    assert!(serde_json::from_value::<super::super::Summary>(unknown).is_err());
}

fn semantic_expected<'a>(
    input: &'a crate::concrete_types::TypeTree,
    output: &'a crate::concrete_types::TypeTree,
    wire: &WireContract,
) -> Contract<'a> {
    Contract {
        input,
        output,
        input_cases: wire.input_cases.clone(),
        output_cases: wire.output_cases.clone(),
        alternatives: wire.alternatives.clone(),
    }
}

#[test]
fn changed_routes_guards_rosters_and_formals_cannot_match_retained_native_contract() {
    // The retained HIR fixture intentionally omits frontend capture options.
    // Use the independently retained original endpoint data, never a fake receipt.
    let endpoints: Value =
        serde_json::from_str(include_str!("../../copy_conversion_members/original.json")).unwrap();
    let input: crate::concrete_types::TypeTree =
        serde_json::from_value(endpoints["tagged"].clone()).unwrap();
    let output = crate::concrete_types::TypeTree::Result {
        ok: Box::new(serde_json::from_value(endpoints["payload"].clone()).unwrap()),
        err: Box::new(serde_json::from_value(endpoints["error"].clone()).unwrap()),
    };
    let wire = original("inverse");
    let expected = semantic_expected(&input, &output, &wire);
    wire.validate_against_contract(&expected).unwrap();

    let source = serde_json::to_value(&wire).unwrap();
    for (pointer, replacement) in [
        ("/alternatives/1/leaves/0/root/parameter", json!(1)),
        ("/alternatives/1/leaves/0/guard/scrutinee/local", json!(999)),
        ("/alternatives/1/condition/value/pattern/local", json!(999)),
        (
            "/alternatives/1/leaves/0/input/0/value/variant",
            json!("InsufficientInformation"),
        ),
        (
            "/alternatives/1/leaves/0/output/0/kind",
            json!("result_err"),
        ),
        ("/output_cases", json!([])),
        ("/input_cases", json!([])),
        ("/alternatives", json!([])),
    ] {
        let mut changed = source.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        let changed: WireContract = serde_json::from_value(changed).unwrap();
        assert!(
            changed.validate_against_contract(&expected).is_err(),
            "{pointer}"
        );
    }
}
