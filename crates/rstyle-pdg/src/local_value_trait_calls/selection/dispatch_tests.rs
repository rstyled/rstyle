use super::*;
use serde_json::json;

fn fixture() -> (HirNode, model::Call) {
    let inventory = super::super::tests::u32_usize::inventory();
    let mut node = super::super::tests::node();
    let call = original_call(&inventory, &node).unwrap();
    node.ty = Some(crate::ResolvedTy {
        normalized: Some("usize".into()),
        observed: "usize".into(),
        spelling: "".into(),
    });
    let mut operand = node.clone();
    operand.kind = "expr_path".into();
    operand.ty.as_mut().unwrap().normalized = Some("u32".into());
    node.children = vec![operand];
    node.res = Some(serde_json::from_value(json!({"kind":"resolved_callee","target":null,
        "description": json!({"path":call.requested.path,"crate":call.requested.crate_name,
            "definition_hash":call.requested.definition_identity,"crate_hash":"local-unencoded"}).to_string()})).unwrap());
    node.dispatch = Some(
        serde_json::from_value(json!({"kind":"resolved_method",
        "op":call.requested.path,"operand_tys":["u32"],"result_ty":"usize"}))
        .unwrap(),
    );
    (node, call)
}

#[test]
fn original_dispatch_must_match_structural_route_and_exact_requested_trait_member() {
    let (node, call) = fixture();
    requested(&node, &call).unwrap();
    for (input, output) in [("usize", "usize"), ("u32", "u64"), ("u16", "usize")] {
        let mut changed = node.clone();
        let dispatch = changed.dispatch.as_mut().unwrap();
        dispatch.operand_tys = vec![input.into()];
        dispatch.result_ty = output.into();
        assert!(requested(&changed, &call).is_err());
    }
    let mut wrong_receiver = node.clone();
    wrong_receiver.children[0].ty.as_mut().unwrap().normalized = Some("usize".into());
    assert!(requested(&wrong_receiver, &call).is_err());
    let mut wrong_result = node.clone();
    wrong_result.ty.as_mut().unwrap().normalized = Some("u64".into());
    assert!(requested(&wrong_result, &call).is_err());
    let mut wrong_method = node.clone();
    wrong_method.dispatch.as_mut().unwrap().op = "util::int::Other::as_usize".into();
    assert!(requested(&wrong_method, &call).is_err());
    let mut static_call = node;
    static_call.kind = "expr_call".into();
    assert!(requested(&static_call, &call).is_err());
}

#[test]
fn unchanged_display_cannot_hide_forged_core_or_selected_resolution() {
    let (node, call) = fixture();
    for (field, value) in [
        ("path", json!(call.selected.path)),
        ("crate", json!("core")),
        ("definition_hash", json!(call.selected.definition_identity)),
        ("crate_hash", json!("invented-hash")),
    ] {
        let mut changed = node.clone();
        let resolution = changed.res.as_mut().unwrap();
        let mut description: Value = serde_json::from_str(&resolution.description).unwrap();
        description[field] = value;
        resolution.description = description.to_string();
        assert!(requested(&changed, &call).is_err(), "{field}");
    }
}
