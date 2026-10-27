use super::*;
use serde_json::{Value, json};

fn witness() -> Value {
    let hir = json!({"krate":0,"owner":1,"local":2});
    let callee = json!({"crate":"core","crate_hash":"hash","definition_hash":"definition","path":"core::selected"});
    let input = "std::result::Result<usize, Error>";
    let output = "std::result::Result<&[u8], Error>";
    let residual = "std::result::Result<!, Error>";
    let control = "std::ops::ControlFlow<std::result::Result<!, Error>, usize>";
    let selection = |args: Value, impl_args: Value, input: &str, output: &str| {
        json!({
            "method":callee,"implementation":callee,"arguments":args,
            "implementation_arguments":impl_args,"input":input,"output":output
        })
    };
    let arm = json!({"arm":hir,"pattern":hir,"binding":hir,"usage":hir,"variant":callee});
    json!({"identity":capture::IDENTITY,"root":hir,"operand":hir,
        "try_span":{"start":0,"end":2},"operand_span":{"start":0,"end":1},
        "branch_call":hir,"residual_call":hir,"early_return":hir,
        "shape":{"input":input,"output":output,"payload":"usize","output_payload":"&[u8]",
            "error":"Error","residual":residual,"control_flow":control},
        "branch":selection(json!([input]), json!(["usize","Error"]), input, control),
        "residual":selection(json!([output,residual]), json!(["&[u8]","Error","Error"]), residual, output),
        "conversion":selection(json!(["Error","Error"]), json!(["Error"]), "Error", "Error"),
        "break_arm":arm,"continue_arm":arm})
}

#[test]
fn independent_scalar_to_slice_signatures_reject_forwarding_and_error_conversion() {
    let value = witness();
    let witness: Witness = serde_json::from_value(value.clone()).unwrap();
    authority::signatures(&witness).unwrap();
    authority::concrete_shape(&witness.shape).unwrap();
    for (pointer, replacement) in [
        ("/residual/output", json!(witness.shape.input)),
        ("/residual/arguments/0", json!(witness.shape.input)),
        ("/residual/implementation_arguments/0", json!("usize")),
        ("/residual/implementation_arguments/2", json!("OtherError")),
        ("/branch/implementation_arguments/0", json!("&[u8]")),
        ("/conversion/arguments/1", json!("OtherError")),
        (
            "/conversion/implementation_arguments/0",
            json!("OtherError"),
        ),
        ("/conversion/input", json!("OtherError")),
    ] {
        let mut tampered = value.clone();
        *tampered.pointer_mut(pointer).unwrap() = replacement;
        let witness = serde_json::from_value(tampered).unwrap();
        assert!(authority::signatures(&witness).is_err(), "{pointer}");
    }
}

#[test]
fn complete_shape_rejects_wrong_residual_success_and_output_order() {
    let witness: Witness = serde_json::from_value(witness()).unwrap();
    for (field, replacement) in [
        ("residual", "std::result::Result<usize, Error>"),
        (
            "control_flow",
            "std::ops::ControlFlow<std::result::Result<!, Error>>",
        ),
        ("output_payload", "(&[u8], usize)"),
        ("payload", "&mut [u8]"),
        ("output", "std::result::Result<(usize, &[u8]), OtherError>"),
    ] {
        let mut value = serde_json::to_value(&witness.shape).unwrap();
        value[field] = json!(replacement);
        let shape = serde_json::from_value(value).unwrap();
        assert!(authority::concrete_shape(&shape).is_err(), "{field}");
    }
}

#[test]
fn scalar_contract_does_not_inherit_unit_or_reference_success_permission() {
    let captured: Witness = serde_json::from_value(witness()).unwrap();
    for payload in ["()", "u32", "isize", "bool", "char", "&[u8]", "&mut [u8]"] {
        let mut shape = captured.shape.clone();
        shape.payload = payload.into();
        shape.input = format!("std::result::Result<{payload}, Error>");
        shape.control_flow =
            format!("std::ops::ControlFlow<std::result::Result<!, Error>, {payload}>");
        assert!(authority::concrete_shape(&shape).is_err(), "{payload}");
    }
    for output in ["usize", "()", "&mut [u8]", "&[u16]", "(usize, &[u8])"] {
        let mut shape = captured.shape.clone();
        shape.output_payload = output.into();
        shape.output = format!("std::result::Result<{output}, Error>");
        assert!(authority::concrete_shape(&shape).is_err(), "{output}");
    }
}

#[test]
fn witness_requires_conversion_and_rejects_unrecognized_evidence() {
    for field in [
        "conversion",
        "branch",
        "residual",
        "continue_arm",
        "break_arm",
    ] {
        let mut value = witness();
        value.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<Witness>(value).is_err(), "{field}");
    }
    let mut value = witness();
    value["shape"]["copy_error"] = json!(true);
    assert!(serde_json::from_value::<Witness>(value).is_err());
}

#[test]
fn named_lifetime_declaration_preserves_exact_generated_erased_substitutions() {
    let mut value = witness();
    value["shape"]["output"] = json!("std::result::Result<&'input [u8], Error>");
    value["shape"]["output_payload"] = json!("&'input [u8]");
    let captured: Witness = serde_json::from_value(value.clone()).unwrap();
    authority::concrete_shape(&captured.shape).unwrap();
    authority::signatures(&captured).unwrap();
    assert_eq!(
        authority::generated_output(&captured.shape).unwrap(),
        "std::result::Result<&[u8], Error>"
    );
    assert_eq!(
        captured.shape.output,
        "std::result::Result<&'input [u8], Error>"
    );
    for payload in [
        "&'input mut [u8]",
        "&'input [u16]",
        "(u32, &'input [u8])",
        "(&'input [u8], usize)",
        "&'input [u8; 4]",
        "(usize, &'input [u8], ())",
        "&'input [u8<'input>]",
    ] {
        let mut tampered = captured.shape.clone();
        tampered.output_payload = payload.into();
        tampered.output = format!("std::result::Result<{payload}, Error>");
        assert!(authority::generated_output(&tampered).is_err(), "{payload}");
    }
    for (pointer, replacement) in [
        (
            "/residual/output",
            json!("std::result::Result<(usize, &[u8]), OtherError>"),
        ),
        (
            "/residual/implementation_arguments/0",
            json!("(usize, &mut [u8])"),
        ),
        (
            "/residual/arguments/0",
            json!("std::result::Result<(&[u8], usize), Error>"),
        ),
        ("/residual/implementation_arguments/2", json!("OtherError")),
    ] {
        let mut tampered = value.clone();
        *tampered.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            authority::signatures(&serde_json::from_value(tampered).unwrap()).is_err(),
            "{pointer}"
        );
    }
}

fn original() -> (HirNode, Witness) {
    let node: HirNode = serde_json::from_str(include_str!("original-try.json")).unwrap();
    let branch = &node.children[0];
    let operand = &branch.children[0];
    let returned = &node.children[1].children[1];
    let residual = &returned.children[0];
    let mut value = witness();
    for (field, hir) in [
        ("root", &node.hir),
        ("operand", &operand.hir),
        ("branch_call", &branch.hir),
        ("residual_call", &residual.hir),
        ("early_return", &returned.hir),
    ] {
        value[field] = json!(hir);
    }
    value["try_span"] = json!(node.span);
    value["operand_span"] = json!(operand.span);
    for (role, arm, usage) in [
        ("break_arm", &node.children[1], &residual.children[0]),
        (
            "continue_arm",
            &node.children[2],
            &node.children[2].children[1],
        ),
    ] {
        value[role]["arm"] = json!(arm.hir);
        value[role]["pattern"] = json!(arm.children[0].hir);
        value[role]["usage"] = json!(usage.hir);
        value[role]["binding"] = json!(usage.res.as_ref().unwrap().target);
    }
    original_shape(&mut value, branch, residual, returned);
    (node, serde_json::from_value(value).unwrap())
}

fn original_shape(value: &mut Value, branch: &HirNode, residual: &HirNode, returned: &HirNode) {
    for (role, call) in [("branch", branch), ("residual", residual)] {
        let dispatch = call.dispatch.as_ref().unwrap();
        value[role]["method"] =
            serde_json::from_str(&call.res.as_ref().unwrap().description).unwrap();
        value[role]["input"] = json!(dispatch.operand_tys[0]);
        value[role]["output"] = json!(dispatch.result_ty);
    }
    value["shape"] = json!({
        "input":branch.dispatch.as_ref().unwrap().operand_tys[0],
        "output":returned.ty.as_ref().unwrap().observed,
        "payload":"usize", "output_payload":"&'i [u8]", "error":"errors::Error",
        "residual":residual.dispatch.as_ref().unwrap().operand_tys[0],
        "control_flow":branch.ty.as_ref().unwrap().observed,
    });
}

#[test]
fn original_scalar_continuation_requires_exact_disjoint_owner_local_bindings() {
    let (node, witness) = original();
    let (operand, returned, _) = structure(&node, &witness).unwrap();
    assert_eq!(
        operand.span,
        crate::ByteSpan {
            start: 86,
            end: 137
        }
    );
    assert_eq!(
        returned.span,
        crate::ByteSpan {
            start: 137,
            end: 138
        }
    );
    for pointer in [
        "/children/0/children/0/hir/local",
        "/children/1/children/1/children/0/children/0/res/target/local",
        "/children/2/children/1/res/target/local",
    ] {
        let mut wire = serde_json::to_value(&node).unwrap();
        *wire.pointer_mut(pointer).unwrap() = json!(9999);
        let changed = serde_json::from_value(wire).unwrap();
        assert!(structure(&changed, &witness).is_err(), "{pointer}");
    }
    for field in ["binding", "owner"] {
        let mut changed = witness.clone();
        if field == "binding" {
            changed.continue_arm.binding = changed.break_arm.binding.clone();
        } else {
            changed.continue_arm.binding.owner += 1;
        }
        assert!(structure(&node, &changed).is_err(), "{field}");
    }
}

#[test]
fn original_generated_tree_rejects_missing_arms_scalar_changes_and_added_effects() {
    let (node, witness) = original();
    for mode in [
        "missing", "extra", "scalar", "guard", "dispatch", "residual",
    ] {
        let mut changed = node.clone();
        match mode {
            "missing" => {
                changed.children.pop();
            }
            "extra" => changed.children.push(node.children[2].clone()),
            "scalar" => {
                changed.children[2].children[1]
                    .ty
                    .as_mut()
                    .unwrap()
                    .normalized = Some("u32".into())
            }
            "guard" => changed.children[1]
                .children
                .push(node.children[2].children[1].clone()),
            "dispatch" => {
                changed.children[0].dispatch.as_mut().unwrap().operand_tys[0] = "OtherResult".into()
            }
            "residual" => {
                changed.children[1].children[1].children[0]
                    .ty
                    .as_mut()
                    .unwrap()
                    .observed = "core::result::Result<usize, errors::Error>".into()
            }
            _ => unreachable!(),
        }
        assert!(structure(&changed, &witness).is_err(), "{mode}");
    }
}
