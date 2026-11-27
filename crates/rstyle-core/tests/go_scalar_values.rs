//! The bundled Go frontend capture contains the complete source. The successful native
//! controls returned 52, 23, 30, 187 and 14. The unbuffered send never finishes.
use rstyle_core::cpg::{bounds::ExactValue, layout, model::NodeKind, records};
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("../src/cpg/testdata/scalars-go.cpg.json")).unwrap()
}

fn result(json: Value) -> std::collections::BTreeMap<String, Option<ExactValue>> {
    let doc = layout::bundle(vec![json]).unwrap();
    let out = records::records(
        &doc,
        records::Demand {
            values: true,
            ..Default::default()
        },
    )
    .unwrap();
    let functions = doc.unit.unwrap().functions.unwrap();
    out.into_iter()
        .zip(functions)
        .map(|(record, function)| {
            let last = function
                .nodes
                .iter()
                .filter(|node| node.kind == NodeKind::Return)
                .max_by_key(|node| node.span.as_ref().map(|span| span.start))
                .unwrap();
            let known = record.exact_values.unwrap();
            (record.name, known.get(&last.children[0]).cloned())
        })
        .collect()
}

#[test]
fn sequential_buffers_are_observed_at_the_return_operand() {
    let values = result(fixture());
    for (name, n) in [
        ("overlap", 52),
        ("overlapCopy", 23),
        ("drain", 30),
        ("bytesCopied", 187),
        ("mapAlias", 14),
        ("receiver", 10),
        ("namedPtr", 10),
        ("exact", 4),
        ("skipped", 2),
    ] {
        assert_eq!(
            values[name],
            Some(ExactValue::Integer(n.to_string())),
            "{name}"
        );
    }
    for name in ["blocked", "unknown"] {
        assert_eq!(values[name], None, "{name}");
    }
}

#[test]
fn a_spread_referring_to_the_wrong_node_cannot_produce_a_value() {
    let mut doc = fixture();
    let calls = doc["unit"]["extensions"]["go:spread-arguments"]
        .as_object_mut()
        .unwrap();
    let target = calls
        .values_mut()
        .find(|v| v.as_array().is_some_and(|a| !a.is_empty()))
        .unwrap();
    *target = serde_json::json!(["unrelated-node"]);
    assert_eq!(result(doc)["overlap"], None);
}

#[test]
fn a_field_bound_to_the_wrong_pointer_target_cannot_produce_a_value() {
    let mut doc = fixture();
    let types = doc["unit"]["types"].as_array_mut().unwrap();
    let int_id = types.iter().find(|t| t["spelling"] == "int").unwrap()["id"].clone();
    let named = types
        .iter_mut()
        .find(|t| t["spelling"] == "example.org/scalars.P")
        .unwrap();
    named["target"] = int_id;
    assert_eq!(result(doc)["namedPtr"], None);
}

fn methods() -> Value {
    serde_json::from_str(include_str!("../src/cpg/testdata/methods-go.cpg.json")).unwrap()
}

#[test]
fn dynamic_and_bound_calls_follow_the_receiver_value() {
    let values = result(methods());
    for (name, n) in [
        ("callA", 6),
        ("callB", 12),
        ("bound", 13),
        ("assertA", 4),
        ("assertB", 3),
    ] {
        assert_eq!(
            values[name],
            Some(ExactValue::Integer(n.to_string())),
            "{name}"
        );
    }
    assert_eq!(values["typedNil"], Some(ExactValue::Boolean(false)));
    assert_eq!(values["incomparable"], None);
}

#[test]
fn a_missing_dynamic_method_target_cannot_select_a_body() {
    let mut input = methods();
    let unit = input["unit"].as_object_mut().unwrap();
    unit["extensions"]
        .as_object_mut()
        .unwrap()
        .remove("go:method-targets");
    let values = result(input);
    assert_eq!(values["callA"], None);
    assert_eq!(values["callB"], None);
    assert_eq!(values["bound"], Some(ExactValue::Integer("13".to_string())));
}

#[test]
fn array_pointer_writes_map_key_types_and_channel_termination_are_preserved() {
    let values = result(
        serde_json::from_str(include_str!(
            "../src/cpg/testdata/array-channel-go.cpg.json"
        ))
        .unwrap(),
    );
    for (name, n) in [("arrayView", 676), ("mapKeys", 239), ("drained", 37)] {
        assert_eq!(
            values[name],
            Some(ExactValue::Integer(n.to_string())),
            "{name}"
        );
    }
    assert_eq!(values["dynamicKey"], Some(ExactValue::Boolean(false)));
    assert_eq!(values["zeroPointer"], Some(ExactValue::Boolean(true)));
    assert_eq!(values["shortSlice"], None);
    assert_eq!(values["openChannel"], None);
}

fn memory_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../src/cpg/testdata/unsafe-memory-go.cpg.json"
    ))
    .unwrap()
}

#[test]
fn memory_views_and_symbolic_addresses_agree_with_native_go() {
    let values = result(memory_fixture());
    for (name, n) in [("shifted", 3), ("window", 25), ("text", 9), ("align", 3)] {
        assert_eq!(
            values[name],
            Some(ExactValue::Integer(n.to_string())),
            "{name}"
        );
    }
    assert_eq!(values["field"], Some(ExactValue::Boolean(true)));
}

#[test]
fn layout_and_operation_controls_change_the_affected_memory_result() {
    let mut doc = memory_fixture();
    let layouts = doc["unit"]["extensions"]["go:address-layouts"]
        .as_object_mut()
        .unwrap();
    let rec = layouts
        .values_mut()
        .find(|lay| lay.get("offsets").is_some())
        .unwrap();
    rec["offsets"][1] = serde_json::json!(0);
    let def = doc["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["operation"] == "go:unsafe.SliceData")
        .unwrap();
    def["standard"] = serde_json::json!(false);
    let values = result(doc);
    assert_eq!(values["field"], Some(ExactValue::Boolean(false)));
    assert_eq!(values["window"], None);
    assert_eq!(values["shifted"], Some(ExactValue::Integer("3".into())));
}
