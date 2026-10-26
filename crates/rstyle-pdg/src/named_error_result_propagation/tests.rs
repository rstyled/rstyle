use super::*;
use serde_json::{Value, json};

fn witness() -> Value {
    let hir = json!({"krate":0,"owner":1,"local":2});
    let callee = json!({"crate":"core","crate_hash":"hash","definition_hash":"definition","path":"core::selected"});
    let input = "std::result::Result<(), Error>";
    let output = "std::result::Result<(u32, usize), Error>";
    let residual = "std::result::Result<!, Error>";
    let control = "std::ops::ControlFlow<std::result::Result<!, Error>>";
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
        "shape":{"input":input,"output":output,"payload":"()","output_payload":"(u32, usize)",
            "error":"Error","residual":residual,"control_flow":control},
        "branch":selection(json!([input]), json!(["()","Error"]), input, control),
        "residual":selection(json!([output,residual]), json!(["(u32, usize)","Error","Error"]), residual, output),
        "conversion":selection(json!(["Error","Error"]), json!(["Error"]), "Error", "Error"),
        "break_arm":arm,"continue_arm":arm})
}

#[test]
fn independent_unit_to_reader_signatures_reject_forwarding_and_error_conversion() {
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
fn complete_shapes_keep_unit_continuation_and_exact_tuple_output() {
    let original = witness();
    let parsed: Witness = serde_json::from_value(original.clone()).unwrap();
    authority::concrete_shape(&parsed.shape).unwrap();
    let mut explicit = parsed.shape.clone();
    explicit.control_flow = format!("std::ops::ControlFlow<{}, ()>", explicit.residual);
    authority::concrete_shape(&explicit).unwrap();
    for (field, replacement) in [
        ("payload", "usize"),
        ("output_payload", "(usize, u32)"),
        ("output_payload", "(u16, usize)"),
        ("output_payload", "(u32, u64)"),
        ("output_payload", "(u32, usize, usize)"),
        ("residual", "std::result::Result<(), Error>"),
        (
            "control_flow",
            "std::ops::ControlFlow<std::result::Result<!, Error>, usize>",
        ),
        ("output", "std::result::Result<(u32, usize), OtherError>"),
    ] {
        let mut value = original["shape"].clone();
        value[field] = json!(replacement);
        let shape = serde_json::from_value(value).unwrap();
        assert!(
            authority::concrete_shape(&shape).is_err(),
            "{field}: {replacement}"
        );
    }
}

#[test]
fn reader_success_vocabulary_excludes_adjacent_scalar_and_product_shapes() {
    let scalar = |name: &str| TypeToken::Scalar(name.into());
    assert!(success(&scalar("unit")));
    assert!(success(&TypeToken::Tuple(vec![
        scalar("u32"),
        scalar("usize")
    ])));
    for token in [
        scalar("usize"),
        scalar("u32"),
        TypeToken::Tuple(vec![]),
        TypeToken::Tuple(vec![scalar("usize"), scalar("u32")]),
        TypeToken::Tuple(vec![scalar("u16"), scalar("usize")]),
        TypeToken::Tuple(vec![scalar("u32"), scalar("u64")]),
        TypeToken::Tuple(vec![scalar("u32"), scalar("usize"), scalar("usize")]),
        TypeToken::Shared(Box::new(scalar("unit"))),
    ] {
        assert!(!success(&token), "{token:?}");
    }
}

#[test]
fn reader_protocol_is_not_historical_scalar_authority() {
    assert_ne!(
        capture::IDENTITY,
        crate::scalar_result_propagation::capture::IDENTITY
    );
    assert_ne!(
        capture::RESOLUTION,
        crate::scalar_result_propagation::capture::RESOLUTION
    );
    assert_ne!(CAPABILITY, crate::scalar_result_propagation::CAPABILITY);
}
