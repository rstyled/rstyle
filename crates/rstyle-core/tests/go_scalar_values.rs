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
                .and_then(|node| node.children.first());
            let known = record.exact_values.unwrap();
            (record.name, last.and_then(|id| known.get(id).cloned()))
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

#[test]
fn instantiated_widths_variadic_storage_and_multi_value_arguments_reach_the_caller() {
    let values = result(
        serde_json::from_str(include_str!("../src/cpg/testdata/frames-go.cpg.json")).unwrap(),
    );
    for (name, n) in [
        ("narrow", 73),
        ("wide", 329),
        ("packet", 67),
        ("shared", 27),
        ("multi", 68),
        ("multiVar", 68),
        ("clean", 44),
    ] {
        assert_eq!(
            values[name],
            Some(ExactValue::Integer(n.to_string())),
            "{name}"
        );
    }
    for (name, b) in [
        ("emptyPack", true),
        ("emptySpread", false),
        ("nilFn", true),
        ("boxedNilFn", false),
        ("literalFn", false),
    ] {
        assert_eq!(values[name], Some(ExactValue::Boolean(b)), "{name}");
    }
    assert_eq!(values["dirty"], Some(ExactValue::Integer("19".into())));
}

#[test]
fn sequenced_pointer_calls_preserve_aliases_and_loop_posts_follow_the_body() {
    let values = result(
        serde_json::from_str(include_str!(
            "../src/cpg/testdata/pointer-frames-go.cpg.json"
        ))
        .unwrap(),
    );
    for (name, n) in [
        ("captured", 48),
        ("expression", 29),
        ("copy", 92),
        ("aliases", 772),
        ("independent", 557),
        ("post", 12),
        ("none", 7),
    ] {
        assert_eq!(
            values[name],
            Some(ExactValue::Integer(n.to_string())),
            "{name}"
        );
    }
    // Go does not require the unrelated read to follow the call in this expression.
    assert_eq!(values["uncertain"], None);
    assert_eq!(values["forever"], None);
}

fn numeric_records() -> Value {
    serde_json::from_str(include_str!(
        "../src/cpg/testdata/go-numeric-records.cpg.json"
    ))
    .unwrap()
}

#[test]
fn ordered_extrema_reflection_and_operand_evaluation_match_native_go() {
    let values = result(numeric_records());
    for (name, n) in [
        ("tags", 2),
        ("promoted", 35),
        ("quotient", 62),
        ("skip", 3),
        ("evaluate", 30),
    ] {
        assert_eq!(
            values[name],
            Some(ExactValue::Integer(n.to_string())),
            "{name}"
        );
    }
    for name in ["first", "sign", "nan"] {
        assert_eq!(values[name], Some(ExactValue::Boolean(true)), "{name}");
    }
}

#[test]
fn only_certified_tag_and_constant_site_facts_produce_their_values() {
    let mut doc = numeric_records();
    let tags = doc["unit"]["extensions"]["go:field-tags"]
        .as_object_mut()
        .unwrap();
    let tag = tags
        .values_mut()
        .find(|v| v["0"] == "6b3a22616222")
        .unwrap();
    tag["0"] = serde_json::json!("6b3a2261626322");
    let functions = doc["unit"]["functions"].as_array_mut().unwrap();
    let f = functions.iter_mut().find(|f| f["name"] == "skip").unwrap();
    f["values"] = serde_json::json!([]);
    let values = result(doc);
    assert_eq!(values["tags"], Some(ExactValue::Integer("3".into())));
    assert_eq!(values["skip"], None);
    assert_eq!(values["evaluate"], Some(ExactValue::Integer("30".into())));
}

#[test]
fn closure_activations_and_sequenced_mutations_preserve_shared_storage() {
    let values = result(
        serde_json::from_str(include_str!(
            "../src/cpg/testdata/closure-frames-go.cpg.json"
        ))
        .unwrap(),
    );
    for (name, n) in [
        ("copies", 34),
        ("separate", 38),
        ("late", 7),
        ("named", 43),
        ("fresh", 39),
        ("resetSlice", 200),
        ("resetMap", 0),
        ("sequencedSlice", 19),
    ] {
        assert_eq!(
            values[name],
            Some(ExactValue::Integer(n.to_string())),
            "{name}"
        );
    }
    assert_eq!(values["unordered"], None);
    assert_eq!(values["unorderedSlice"], None);
    // A saved pointer method can carry a different range variable from each iteration.
    assert_eq!(values["escapedIterations"], None);
}

#[test]
fn receiver_and_method_widths_are_independent_and_zero_size_keys_stay_unknown() {
    let doc: Value = serde_json::from_str(include_str!(
        "../src/cpg/testdata/method-instances-go.cpg.json"
    ))
    .unwrap();
    let values = result(doc.clone());
    for (name, n) in [("wideReceiver", 256), ("wideMethod", 512), ("keyValue", 7)] {
        assert_eq!(
            values[name],
            Some(ExactValue::Integer(n.to_string())),
            "{name}"
        );
    }
    assert_eq!(values["zeroKey"], None);
    let mut changed = doc;
    let u = changed["unit"].as_object_mut().unwrap();
    let wrong = u["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["spelling"] == "uint8")
        .unwrap()["id"]
        .clone();
    let f = u["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|f| f["name"] == "wideMethod")
        .unwrap();
    let call = f["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["dispatch"] == "static")
        .unwrap();
    call["instance_arguments"][0] = wrong;
    assert_eq!(result(changed)["wideMethod"], None);
}
