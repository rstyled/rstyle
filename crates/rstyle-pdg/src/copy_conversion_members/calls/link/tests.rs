//! Link shape is only observational; these rows supply no callee authority.
use super::*;
use crate::data_types::TypeToken;

#[test]
fn original_call_output_controls_candidate_shape_independently_of_caller_result() {
    let values: Vec<Value> = serde_json::from_str(include_str!("../original-calls.json")).unwrap();
    for value in values {
        let node: HirNode = serde_json::from_value(value["node"].clone()).unwrap();
        let role = if value["name"] == "wrap" {
            "from-method"
        } else {
            "tryfrom-method"
        };
        candidate_types(&node.children[0], &node, role).unwrap();

        let mut scalar = node.clone();
        scalar.ty.as_mut().unwrap().normalized = Some("bool".into());
        assert!(candidate_types(&node.children[0], &scalar, role).is_err());
        assert!(candidate_types(&node.children[0], &node, "another-member").is_err());
    }
}

#[test]
fn nonempty_error_layout_cannot_be_a_tagged_conversion_link() {
    let values: Vec<Value> = serde_json::from_str(include_str!("../original-calls.json")).unwrap();
    let value = values
        .iter()
        .find(|value| value["name"] == "inverse")
        .unwrap();
    let node: HirNode = serde_json::from_value(value["node"].clone()).unwrap();
    let TypeToken::Result { ok, .. } = crate::data_aggregates::parsed_type(&node).unwrap() else {
        unreachable!()
    };
    let changed = crate::data_types::TypeToken::Result {
        ok: ok.clone(),
        err: ok,
    };
    let mut changed_node = node.clone();
    changed_node.ty.as_mut().unwrap().normalized = Some(changed.text().unwrap());

    assert!(candidate_types(&node.children[0], &changed_node, "tryfrom-method").is_err());
}
