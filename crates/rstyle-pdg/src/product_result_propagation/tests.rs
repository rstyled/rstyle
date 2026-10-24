use super::*;
use serde_json::{Value, json};

fn witness() -> Value {
    let hir = json!({"krate":0,"owner":1,"local":2});
    let callee = json!({"crate":"core","crate_hash":"hash","definition_hash":"definition","path":"core::selected"});
    let input = "core::result::Result<decoder::LineReader<'_>, errors::Error>";
    let output = "core::result::Result<decoder::LineReader<'i>, errors::Error>";
    let payload = "decoder::LineReader<'_>";
    let error = "errors::Error";
    let residual = "core::result::Result<!, errors::Error>";
    let control =
        "core::ops::ControlFlow<core::result::Result<!, errors::Error>, decoder::LineReader<'_>>";
    let selection = |args: Value, impl_args: Value, input: &str, output: &str| {
        json!({"method":callee,"implementation":callee,"arguments":args,
            "implementation_arguments":impl_args,"input":input,"output":output})
    };
    let arm = json!({"arm":hir,"pattern":hir,"binding":hir,"usage":hir,"variant":callee});
    json!({"identity":capture::IDENTITY,"root":hir,"operand":hir,
        "try_span":{"start":0,"end":2},"operand_span":{"start":0,"end":1},
        "branch_call":hir,"residual_call":hir,"early_return":hir,
        "shape":{"input":input,"output":output,"payload":payload,"output_payload":"decoder::LineReader<'i>",
            "error":error,"residual":residual,"control_flow":control},
        "branch":selection(json!([input]),json!([payload,error]),input,control),
        "residual":selection(json!([input,residual]),json!([payload,error,error]),residual,input),
        "conversion":selection(json!([error,error]),json!([error]),error,error),
        "break_arm":arm,"continue_arm":arm})
}

#[test]
fn exact_product_signatures_preserve_generated_and_declared_observations() {
    let captured: Witness = serde_json::from_value(witness()).unwrap();
    authority::signatures(&captured).unwrap();
    assert_eq!(
        shape::generated_output(&captured.shape).unwrap(),
        captured.shape.input
    );
    assert_ne!(captured.shape.input, captured.shape.output);
}

#[test]
fn product_identity_signature_and_conversion_substitutions_are_independent() {
    let original = witness();
    for (pointer, replacement) in [
        (
            "/residual/output",
            json!("core::result::Result<decoder::LineReader<'i>, errors::Error>"),
        ),
        (
            "/residual/implementation_arguments/0",
            json!("decoder::Other<'_>"),
        ),
        ("/residual/implementation_arguments/2", json!("OtherError")),
        (
            "/branch/implementation_arguments/0",
            json!("decoder::LineReader<'static>"),
        ),
        ("/conversion/arguments/1", json!("OtherError")),
        (
            "/conversion/implementation_arguments/0",
            json!("OtherError"),
        ),
        ("/conversion/input", json!("OtherError")),
        (
            "/branch/input",
            json!("core::result::Result<(), errors::Error>"),
        ),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            authority::signatures(&serde_json::from_value(changed).unwrap()).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn malformed_product_protocol_shape_never_grants_extraction() {
    let original = witness()["shape"].clone();
    for (field, replacement) in [
        ("payload", "decoder::LineReader<'static>"),
        ("payload", "decoder::LineReader<u8>"),
        ("output_payload", "decoder::Other<'i>"),
        ("output_payload", "decoder::LineReader<'i, 'j>"),
        ("output_payload", "&'i [u8]"),
        ("residual", "core::result::Result<usize, errors::Error>"),
        (
            "control_flow",
            "core::ops::ControlFlow<(), decoder::LineReader<'_>>",
        ),
    ] {
        let mut changed = original.clone();
        changed[field] = json!(replacement);
        assert!(
            shape::generated_output(&serde_json::from_value(changed).unwrap()).is_err(),
            "{field}: {replacement}"
        );
    }
}
