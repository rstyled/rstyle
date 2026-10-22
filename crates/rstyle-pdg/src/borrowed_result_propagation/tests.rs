use super::*;
use serde_json::{Value, json};

fn witness() -> Value {
    let hir = json!({"krate":0,"owner":1,"local":2});
    let callee = json!({"crate":"core","crate_hash":"hash","definition_hash":"definition","path":"core::selected"});
    let input = "std::result::Result<&[u8], Error>";
    let output = "std::result::Result<(usize, &[u8]), Error>";
    let residual = "std::result::Result<!, Error>";
    let control = "std::ops::ControlFlow<std::result::Result<!, Error>, &[u8]>";
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
        "shape":{"input":input,"output":output,"payload":"&[u8]","output_payload":"(usize, &[u8])",
            "error":"Error","residual":residual,"control_flow":control},
        "branch":selection(json!([input]), json!(["&[u8]","Error"]), input, control),
        "residual":selection(json!([output,residual]), json!(["(usize, &[u8])","Error","Error"]), residual, output),
        "conversion":selection(json!(["Error","Error"]), json!(["Error"]), "Error", "Error"),
        "break_arm":arm,"continue_arm":arm})
}

#[test]
fn independent_slice_to_product_signatures_reject_forwarding_and_error_conversion() {
    let value = witness();
    let witness: Witness = serde_json::from_value(value.clone()).unwrap();
    authority::signatures(&witness).unwrap();
    authority::concrete_shape(&witness.shape).unwrap();
    for (pointer, replacement) in [
        ("/residual/output", json!(witness.shape.input)),
        ("/residual/arguments/0", json!(witness.shape.input)),
        ("/residual/implementation_arguments/0", json!("&[u8]")),
        ("/residual/implementation_arguments/2", json!("OtherError")),
        (
            "/branch/implementation_arguments/0",
            json!("(usize, &[u8])"),
        ),
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
        ("residual", "std::result::Result<&[u8], Error>"),
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
    value["shape"]["output"] = json!("std::result::Result<(usize, &'input [u8]), Error>");
    value["shape"]["output_payload"] = json!("(usize, &'input [u8])");
    let captured: Witness = serde_json::from_value(value.clone()).unwrap();
    authority::concrete_shape(&captured.shape).unwrap();
    authority::signatures(&captured).unwrap();
    assert_eq!(
        authority::generated_output(&captured.shape).unwrap(),
        "std::result::Result<(usize, &[u8]), Error>"
    );
    assert_eq!(
        captured.shape.output,
        "std::result::Result<(usize, &'input [u8]), Error>"
    );
    for payload in [
        "(usize, &'input mut [u8])",
        "(usize, &'input [u16])",
        "(u32, &'input [u8])",
        "(&'input [u8], usize)",
        "(usize, &'input [u8; 4])",
        "(usize, &'input [u8], ())",
        "(usize, &'input [u8<'input>])",
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
