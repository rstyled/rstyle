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
