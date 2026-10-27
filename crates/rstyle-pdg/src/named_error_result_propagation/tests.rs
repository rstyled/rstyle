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
fn u16_reader_receipt_requires_a_distinct_protocol_and_consistent_substitutions() {
    let mut value: Value =
        serde_json::from_str(&witness().to_string().replace("u32", "u16")).unwrap();
    value["identity"] = json!(capture::U16_IDENTITY);
    let parsed: Witness = serde_json::from_value(value.clone()).unwrap();
    authority::signatures(&parsed).unwrap();
    assert!(authority::concrete_shape(&parsed.shape).is_err());

    for (pointer, replacement) in [
        ("/identity", json!(capture::IDENTITY)),
        ("/shape/output_payload", json!("(u32, usize)")),
        ("/shape/payload", json!("(u16, usize)")),
        (
            "/residual/implementation_arguments/0",
            json!("(u32, usize)"),
        ),
        ("/residual/implementation_arguments/2", json!("OtherError")),
        ("/branch/implementation_arguments/0", json!("u16")),
    ] {
        let mut changed = value.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        let parsed = serde_json::from_value(changed).unwrap();
        assert!(authority::signatures(&parsed).is_err(), "{pointer}");
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

fn validator_witness() -> Value {
    let mut value = witness();
    let input = "std::result::Result<(u32, usize), Error>";
    let output = "std::result::Result<usize, Error>";
    let control = "std::ops::ControlFlow<std::result::Result<!, Error>, (u32, usize)>";
    value["identity"] = json!(capture::VALIDATOR_IDENTITY);
    value["shape"] = json!({
        "input":input, "output":output, "payload":"(u32, usize)", "output_payload":"usize",
        "error":"Error", "residual":"std::result::Result<!, Error>", "control_flow":control,
    });
    value["branch"]["arguments"] = json!([input]);
    value["branch"]["implementation_arguments"] = json!(["(u32, usize)", "Error"]);
    value["branch"]["input"] = json!(input);
    value["branch"]["output"] = json!(control);
    value["residual"]["arguments"][0] = json!(output);
    value["residual"]["implementation_arguments"][0] = json!("usize");
    value["residual"]["output"] = json!(output);
    value
}

#[test]
fn tuple_to_scalar_validator_pins_actual_payload_and_reflexive_error_conversion() {
    let value = validator_witness();
    let parsed: Witness = serde_json::from_value(value.clone()).unwrap();
    authority::signatures(&parsed).unwrap();
    authority::concrete_shape_for(&parsed.shape, Mode::ReaderToScalar).unwrap();
    assert!(authority::concrete_shape_for(&parsed.shape, Mode::UnitToReader).is_err());
    for (pointer, replacement) in [
        ("/identity", json!(capture::IDENTITY)),
        ("/branch/implementation_arguments/0", json!("()")),
        (
            "/branch/output",
            json!("std::ops::ControlFlow<std::result::Result<!, Error>>"),
        ),
        ("/residual/arguments/0", json!(parsed.shape.input)),
        ("/residual/output", json!(parsed.shape.input)),
        (
            "/residual/implementation_arguments/0",
            json!("(u32, usize)"),
        ),
        ("/residual/implementation_arguments/2", json!("OtherError")),
        ("/conversion/arguments/1", json!("OtherError")),
        ("/conversion/input", json!("OtherError")),
    ] {
        let mut changed = value.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            authority::signatures(&serde_json::from_value(changed).unwrap()).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn validator_continuation_cannot_default_to_unit_or_reorder_tuple_components() {
    let original = validator_witness();
    for (field, replacement) in [
        ("payload", "()"),
        ("payload", "(usize, u32)"),
        ("payload", "(u16, usize)"),
        ("payload", "(u32, u64)"),
        ("payload", "(u32, usize, usize)"),
        ("output_payload", "u32"),
        ("output_payload", "()"),
        ("output_payload", "(u32, usize)"),
        (
            "control_flow",
            "std::ops::ControlFlow<std::result::Result<!, Error>>",
        ),
        (
            "control_flow",
            "std::ops::ControlFlow<std::result::Result<!, Error>, ()>",
        ),
        ("output", "std::result::Result<usize, OtherError>"),
        ("residual", "std::result::Result<(u32, usize), Error>"),
    ] {
        let mut changed = original["shape"].clone();
        changed[field] = json!(replacement);
        assert!(
            authority::concrete_shape_for(
                &serde_json::from_value(changed).unwrap(),
                Mode::ReaderToScalar
            )
            .is_err(),
            "{field}: {replacement}"
        );
    }
}

#[test]
fn transitions_keep_the_historical_success_vocabulary_and_document_meanings_separate() {
    let unit = TypeToken::Scalar("unit".into());
    let scalar = TypeToken::Scalar("usize".into());
    let tuple = TypeToken::Tuple(vec![TypeToken::Scalar("u32".into()), scalar.clone()]);
    assert!(payloads_match(Mode::UnitToReader, &unit, &tuple));
    assert!(payloads_match(Mode::ReaderToScalar, &tuple, &scalar));
    for (input, output) in [
        (&unit, &scalar),
        (&tuple, &tuple),
        (&scalar, &scalar),
        (&scalar, &tuple),
    ] {
        assert!(!payloads_match(Mode::ReaderToScalar, input, output));
    }
    assert!(!success(&scalar));
    assert_ne!(
        Mode::UnitToReader.identity(),
        Mode::ReaderToScalar.identity()
    );
    assert_ne!(
        Mode::UnitToReader.resolution(),
        Mode::ReaderToScalar.resolution()
    );
    assert_ne!(
        Mode::UnitToReader.capability(),
        Mode::ReaderToScalar.capability()
    );
    assert!(
        Mode::ReaderToScalar
            .constructs()
            .iter()
            .all(|construct| !Mode::UnitToReader.constructs().contains(construct))
    );
}

fn product_witness() -> Value {
    let mut value = validator_witness();
    let output = "std::result::Result<(dfa::dense::Flags, usize), Error>";
    value["identity"] = json!(capture::PRODUCT_IDENTITY);
    value["shape"]["output"] = json!(output);
    value["shape"]["output_payload"] = json!("(dfa::dense::Flags, usize)");
    value["residual"]["arguments"][0] = json!(output);
    value["residual"]["implementation_arguments"][0] = json!("(dfa::dense::Flags, usize)");
    value["residual"]["output"] = json!(output);
    value
}

#[test]
fn product_reader_requires_exact_tuple_residual_and_same_error_conversion() {
    let value = product_witness();
    let parsed: Witness = serde_json::from_value(value.clone()).unwrap();
    authority::signatures(&parsed).unwrap();
    for (pointer, replacement) in [
        ("/identity", json!(capture::VALIDATOR_IDENTITY)),
        ("/identity", json!(capture::IDENTITY)),
        ("/branch/implementation_arguments/0", json!("()")),
        ("/residual/arguments/0", json!(parsed.shape.input)),
        ("/residual/output", json!(parsed.shape.input)),
        ("/residual/implementation_arguments/0", json!("usize")),
        ("/residual/implementation_arguments/2", json!("OtherError")),
        ("/conversion/arguments/1", json!("OtherError")),
        (
            "/conversion/implementation_arguments/0",
            json!("OtherError"),
        ),
        ("/conversion/output", json!("OtherError")),
        (
            "/shape/control_flow",
            json!("std::ops::ControlFlow<std::result::Result<!, Error>>"),
        ),
    ] {
        let mut changed = value.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            authority::signatures(&serde_json::from_value(changed).unwrap()).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn product_reader_observed_shape_cannot_authorize_other_payload_grammars() {
    let original: Witness = serde_json::from_value(product_witness()).unwrap();
    for payload in [
        "Flags",
        "(Flags, u64)",
        "(usize, Flags)",
        "(Flags, usize, usize)",
        "(&Flags, usize)",
        "(Flags<u8>, usize)",
        "(<T as Trait>::Flags, usize)",
        "([bool; 3], usize)",
        "((bool, bool, bool), usize)",
    ] {
        let mut shape = original.shape.clone();
        shape.output_payload = payload.into();
        shape.output = format!("std::result::Result<{payload}, Error>");
        assert!(
            authority::concrete_shape_for(&shape, Mode::ReaderToCopyProduct).is_err(),
            "{payload}"
        );
    }
    for mode in [Mode::UnitToReader, Mode::ReaderToScalar] {
        assert!(authority::concrete_shape_for(&original.shape, mode).is_err());
        assert_ne!(mode.capability(), Mode::ReaderToCopyProduct.capability());
        assert_ne!(mode.identity(), Mode::ReaderToCopyProduct.identity());
        assert_ne!(mode.resolution(), Mode::ReaderToCopyProduct.resolution());
        assert!(
            Mode::ReaderToCopyProduct
                .constructs()
                .iter()
                .all(|item| !mode.constructs().contains(item))
        );
    }
    // These spelling checks provide no product ownership or Copy authority.
    // Full facts-stage checks must independently identify the exact declaration.
    for payload in ["(Flags, usize)", "(dfa::dense::Flags, usize)"] {
        let mut shape = original.shape.clone();
        shape.output_payload = payload.into();
        shape.output = format!("std::result::Result<{payload}, Error>");
        authority::concrete_shape_for(&shape, Mode::ReaderToCopyProduct).unwrap();
    }
}

fn id_reader_witness() -> Value {
    let mut value = product_witness();
    let input = "std::result::Result<util::primitives::StateID, Error>";
    let output = "std::result::Result<(util::primitives::StateID, usize), Error>";
    let payload = "util::primitives::StateID";
    let control = format!("std::ops::ControlFlow<std::result::Result<!, Error>, {payload}>");
    value["identity"] = json!(capture::ID_READER_IDENTITY);
    value["shape"]["input"] = json!(input);
    value["shape"]["output"] = json!(output);
    value["shape"]["payload"] = json!(payload);
    value["shape"]["output_payload"] = json!("(util::primitives::StateID, usize)");
    value["shape"]["control_flow"] = json!(control);
    value["branch"]["arguments"] = json!([input]);
    value["branch"]["implementation_arguments"] = json!([payload, "Error"]);
    value["branch"]["input"] = json!(input);
    value["branch"]["output"] = json!(control);
    value["residual"]["arguments"][0] = json!(output);
    value["residual"]["implementation_arguments"][0] = json!("(util::primitives::StateID, usize)");
    value["residual"]["output"] = json!(output);
    value
}

#[test]
fn id_reader_propagation_preserves_nominal_identity_and_error_conversion() {
    let original = id_reader_witness();
    let parsed: Witness = serde_json::from_value(original.clone()).unwrap();
    authority::signatures(&parsed).unwrap();
    for mode in [
        Mode::UnitToReader,
        Mode::ReaderToScalar,
        Mode::ReaderToCopyProduct,
    ] {
        assert!(authority::concrete_shape_for(&parsed.shape, mode).is_err());
        assert_ne!(mode.capability(), Mode::IdToReader.capability());
        assert_ne!(mode.identity(), Mode::IdToReader.identity());
        assert_ne!(mode.resolution(), Mode::IdToReader.resolution());
    }
    for (pointer, replacement) in [
        ("/identity", json!(capture::PRODUCT_IDENTITY)),
        (
            "/shape/output_payload",
            json!("(util::primitives::PatternID, usize)"),
        ),
        ("/shape/payload", json!("usize")),
        ("/branch/implementation_arguments/0", json!("u32")),
        ("/residual/implementation_arguments/2", json!("OtherError")),
        ("/conversion/input", json!("OtherError")),
        ("/residual/output", json!(parsed.shape.input)),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            authority::signatures(&serde_json::from_value(changed).unwrap()).is_err(),
            "{pointer}"
        );
    }
}
