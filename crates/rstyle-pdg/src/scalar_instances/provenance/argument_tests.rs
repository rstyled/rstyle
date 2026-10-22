//! Expectations follow the declared T/T -> i32/i32 contract independently.
use super::*;
use serde_json::{Value, json};

fn original() -> Vec<Parameter> {
    serde_json::from_value(json!([
        {"kind":"parameter","index":0,"name":"T"},
        {"kind":"parameter","index":0,"name":"T"}
    ]))
    .unwrap()
}

fn scalar(name: &str) -> Value {
    json!({"kind":"type","value":"diagnostic only","display_only":true,
        "identity_status":"known","identity":{"kind":"builtin","name":name}})
}

fn check(original: Vec<Parameter>, substituted: Value) -> Result<(), String> {
    let substituted: Vec<Argument> = serde_json::from_value(substituted).unwrap();
    arguments(
        &original,
        &substituted,
        Arguments {
            name: "T",
            scalar: "i32",
            count: 2,
        },
    )
}

#[test]
fn bound_substitution_requires_both_original_parameter_identities_and_both_scalar_types() {
    assert!(check(original(), json!([scalar("i32"), scalar("i32")])).is_ok());
    assert!(check(original(), json!([scalar("i32"), scalar("u32")])).is_err());
    assert!(check(original(), json!([scalar("i32")])).is_err());

    let mut parameters = original();
    parameters[1].index = 1;
    assert!(check(parameters, json!([scalar("i32"), scalar("i32")])).is_err());

    let mut parameters = original();
    parameters[1].name = "Other".into();
    assert!(check(parameters, json!([scalar("i32"), scalar("i32")])).is_err());
}

#[test]
fn unsupported_or_display_only_type_identity_never_supplies_bound_authority() {
    let mut unknown = scalar("i32");
    unknown["identity_status"] = json!("unknown");
    assert!(check(original(), json!([scalar("i32"), unknown])).is_err());

    let mut display = scalar("u32");
    display["value"] = json!("i32");
    assert!(check(original(), json!([scalar("i32"), display])).is_err());
    assert!(
        serde_json::from_value::<Argument>(json!({"kind":"type","value":"i32",
        "display_only":true,"identity_status":"known"}))
        .is_err()
    );
}
