use super::*;

#[test]
fn question_mark_preserves_the_entire_operand() {
    for text in ["haystack.get(span.start)?", "(haystack.get(span.start))?"] {
        let expression: syn::Expr = syn::parse_str(text).unwrap();
        source::question_mark(&expression).unwrap();
    }
    for text in [
        "haystack.get(span.start)",
        "Some(haystack)",
        "#[other] haystack.get(0)?",
    ] {
        let expression: syn::Expr = syn::parse_str(text).unwrap();
        assert!(source::question_mark(&expression).is_err(), "{text}");
    }
}

fn shape() -> capture::Shape {
    capture::Shape {
        input: "std::option::Option<&u8>".into(),
        output: "std::option::Option<Span>".into(),
        output_payload: "Span".into(),
        payload: "&u8".into(),
        residual: "std::option::Option<!>".into(),
        control_flow: "std::ops::ControlFlow<std::option::Option<!>, &u8>".into(),
    }
}

#[test]
fn residual_and_output_shape_are_independently_typed() {
    authority::concrete_shape(&shape()).unwrap();
    for mutation in 0..5 {
        let mut changed = shape();
        match mutation {
            0 => changed.residual = "std::option::Option<core::convert::Infallible>".into(),
            1 => changed.control_flow = "std::ops::ControlFlow<std::option::Option<!>, u8>".into(),
            2 => changed.output_payload = "&u8".into(),
            3 => changed.input = "custom::Option<&u8>".into(),
            _ => changed.output = "std::option::Option<&u8>".into(),
        }
        assert!(authority::concrete_shape(&changed).is_err());
    }
}

fn witness() -> capture::Witness {
    let callee = serde_json::json!({"crate":"core", "crate_hash":"hash", "definition_hash":"definition", "path":"method"});
    let location = serde_json::json!({"krate":0,"owner":1,"local":2});
    let selection = |arguments, implementation_arguments, input, output| {
        serde_json::json!({
            "method": callee, "implementation": callee,
            "arguments": arguments, "implementation_arguments": implementation_arguments,
            "input": input, "output": output,
        })
    };
    let arm = serde_json::json!({"arm":location,"pattern":location,"binding":location,"usage":location,"variant":callee});
    let shape = shape();
    serde_json::from_value(serde_json::json!({
        "identity":capture::IDENTITY,"root":location,"operand":location,
        "try_span":{"start":0,"end":1},"operand_span":{"start":0,"end":1},
        "branch_call":location,"residual_call":location,"early_return":location,
        "branch":selection(vec![shape.input.clone()],vec![shape.payload.clone()],shape.input.clone(),shape.control_flow.clone()),
        "residual":selection(vec![shape.output.clone(),shape.residual.clone()],vec![shape.output_payload.clone()],shape.residual.clone(),shape.output.clone()),
        "shape":shape,"break_arm":arm,"continue_arm":arm,
    })).unwrap()
}

#[test]
fn generic_substitutions_cannot_forward_the_input_as_output() {
    authority::signatures(&witness()).unwrap();
    for mutation in 0..5 {
        let mut changed = witness();
        match mutation {
            0 => changed.residual.output = changed.shape.input.clone(),
            1 => changed.residual.implementation_arguments = vec![changed.shape.payload.clone()],
            2 => changed.branch.implementation_arguments = vec!["u8".into()],
            3 => changed.residual.arguments.reverse(),
            _ => changed.branch.output = changed.shape.residual.clone(),
        }
        assert!(authority::signatures(&changed).is_err());
    }
}

#[test]
fn supplemental_option_witness_does_not_accept_missing_selection_evidence() {
    for text in [
        "{}",
        "{\"identity\":\"rstyle.rust.concrete-option-propagation/1\"}",
    ] {
        assert!(serde_json::from_str::<capture::Witness>(text).is_err());
    }
    let mut record = serde_json::to_value(witness()).unwrap();
    record["conversion"] = serde_json::json!({});
    assert!(serde_json::from_value::<capture::Witness>(record).is_err());
}
