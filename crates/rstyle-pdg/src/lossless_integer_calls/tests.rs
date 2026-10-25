//! Fixed semantic expectations and hostile observations are specified independently.
use super::*;
use serde_json::{Value, json};
mod u32_usize;

fn registry() -> crate::data_library::Registry {
    let mut registry: crate::data_library::Registry =
        serde_json::from_str(include_str!("../../contracts/read-only-lengths-v2.json")).unwrap();
    for (key, value) in [
        (CONVERT, "try-trait"),
        (IMPLEMENTATION, "exact-impl"),
        (U32_USIZE_IMPLEMENTATION, "exact-u32-usize-impl"),
        (EXPECT, "expect-method"),
        (ERROR, "actual-error"),
        (authority::STABLE_CORE, "selected-core"),
    ] {
        registry.definitions.insert(key.into(), value.into());
    }
    for (file, digest) in authority::SOURCES {
        registry
            .source_digests
            .insert((*file).into(), (*digest).into());
    }
    registry
}

fn definition(path: &str, identity: &str) -> Value {
    json!({"path":path, "definition_identity":identity, "crate":"core",
        "stable_crate_id":"selected-core", "local":false})
}

fn builtin(name: &str) -> Value {
    json!({"kind":"type", "value":"not authority", "display_only":true,
        "identity_status":"known", "identity":{"kind":"builtin", "name":name}})
}

fn call(kind: Kind) -> Value {
    let conversion = kind.is_conversion();
    let mut arguments = json!([builtin(kind.output()), builtin(kind.input())]);
    if !conversion {
        arguments[1]["identity"] = json!({"kind":"adt", "arguments":[],
            "definition":definition(ERROR,"actual-error")});
    }
    let (requested, selected) = if conversion {
        (
            definition(CONVERT, "try-trait"),
            definition(
                kind.implementation(),
                if kind.conversion() == Kind::Convert {
                    "exact-impl"
                } else {
                    "exact-u32-usize-impl"
                },
            ),
        )
    } else {
        let definition = definition(EXPECT, "expect-method");
        (definition.clone(), definition)
    };
    let result = format!(
        "core::result::Result<{}, core::num::TryFromIntError>",
        kind.output()
    );
    json!({"hir":{"krate":0,"owner":4,"local":8}, "span":{"start":10,"end":24},
        "status":"resolved", "expanded":false, "requested":requested, "selected":selected,
        "requested_arguments":arguments,
        "selected_arguments":if conversion {json!([])} else {arguments.clone()},
        "signature":{"display_only":true,
            "inputs":if conversion {json!([kind.input()])} else {json!([result.as_str(),"&str"])},
            "output":if conversion {result.as_str()} else {kind.output()}}})
}

fn node() -> HirNode {
    serde_json::from_value(json!({
        "hir":{"krate":0,"owner":4,"local":8},"span":{"start":10,"end":24},
        "kind":"expr_call","snippet":"u64::try_from(n)", "ty":null,
        "res":null,"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,
        "lit":null,"param_index":null,"is_unsafe_block":false,"children":[]
    }))
    .unwrap()
}

fn inventory() -> Value {
    json!({"identity":"rstyle.rust.call-instance-observations/1", "complete":true,
        "calls":[call(Kind::Convert)]})
}

#[test]
fn raw_conversion_and_expect_require_distinct_exact_selected_substitutions() {
    let registry = registry();
    for kind in [
        Kind::Convert,
        Kind::Expect,
        Kind::ConvertU32Usize,
        Kind::ExpectUsize,
    ] {
        assert!(selection::relationship(&call(kind), kind, &registry).is_ok());
        for field in ["requested", "selected"] {
            for (key, value) in [
                ("path", json!("core::convert::From::from")),
                ("definition_identity", json!("another-definition")),
                ("crate", json!("custom")),
                ("stable_crate_id", json!("different-core")),
                ("local", json!(true)),
            ] {
                let mut forged = call(kind);
                forged[field][key] = value;
                assert!(selection::relationship(&forged, kind, &registry).is_err());
            }
        }
    }
}

#[test]
fn display_names_cannot_substitute_for_builtin_or_actual_error_identity() {
    let registry = registry();
    for (pointer, value) in [
        ("/requested_arguments/0/identity/name", json!("u32")),
        ("/requested_arguments/0/identity_status", json!("unknown")),
        ("/requested_arguments/1/identity/name", json!("u32")),
        ("/selected_arguments", json!([builtin("u64")])),
        ("/signature/output", json!("u64")),
    ] {
        let mut forged = call(Kind::Convert);
        *forged.pointer_mut(pointer).unwrap() = value;
        assert!(selection::relationship(&forged, Kind::Convert, &registry).is_err());
    }
    let mut forged = call(Kind::Expect);
    for field in ["requested_arguments", "selected_arguments"] {
        forged[field][1]["identity"]["definition"] =
            definition("std::convert::Infallible", "actual-error");
    }
    assert!(selection::relationship(&forged, Kind::Expect, &registry).is_err());
}

#[test]
fn original_site_cannot_be_missing_duplicated_incomplete_expanded_or_rebound() {
    assert!(selection::site(&inventory(), &node()).is_ok());
    for (pointer, value) in [
        ("/complete", json!(false)),
        ("/identity", json!("older-inventory")),
        ("/calls/0/status", json!("unknown")),
        ("/calls/0/expanded", json!(true)),
        ("/calls/0/hir/local", json!(9)),
        ("/calls/0/span/end", json!(25)),
        ("/calls", json!([])),
        ("/calls", json!([call(Kind::Convert), call(Kind::Convert)])),
    ] {
        let mut forged = inventory();
        *forged.pointer_mut(pointer).unwrap() = value;
        assert!(selection::site(&forged, &node()).is_err(), "{pointer}");
    }
}

#[test]
fn totality_requires_all_reviewed_conversion_error_and_expect_source_pins() {
    let registry = registry();
    assert!(authority::sources(&registry).is_ok());
    for (path, _) in authority::SOURCES {
        let mut missing = self::registry();
        missing.source_digests.remove(*path);
        assert!(authority::sources(&missing).is_err());
        missing
            .source_digests
            .insert((*path).into(), "changed".into());
        assert!(authority::sources(&missing).is_err());
    }
}

#[test]
fn numerical_width_inequality_is_not_reviewed_target_authority() {
    let registry = registry();
    let mut width = crate::CountFact {
        status: crate::FactStatus::Known,
        value: Some(64),
        reason: None,
        policy: None,
    };
    assert!(authority::target(&width, &registry).is_ok());
    for value in [None, Some(16), Some(32), Some(128)] {
        width.value = value;
        assert!(authority::target(&width, &registry).is_err());
    }
    width.value = Some(64);
    width.status = crate::FactStatus::Unknown;
    assert!(authority::target(&width, &registry).is_err());
    width.status = crate::FactStatus::Known;
    let mut wrong_registry = self::registry();
    wrong_registry.pointer_width = 32;
    assert!(authority::target(&width, &wrong_registry).is_err());
}

fn source_arguments(bytes: &str, kind: Kind) -> Result<usize, String> {
    let expression = syn::parse_str(bytes).map_err(|error| format!("{error}"))?;
    source::operands(&expression, kind).map(|operands| operands.len())
}

#[test]
fn original_source_retains_literal_message_and_rejects_effectful_messages() {
    assert_eq!(
        source_arguments("u64::try_from(n)", Kind::Convert).unwrap(),
        1
    );
    assert_eq!(
        source_arguments("u64::try_from(n).expect(\"overflow\")", Kind::Expect).unwrap(),
        2
    );
    for bytes in [
        "u32::try_from(n)",
        "Custom::try_from(n)",
        "u64::try_from::<usize>(n)",
        "u64::try_from(n, other)",
        "<u64 as TryFrom<usize>>::try_from(n)",
    ] {
        assert!(source_arguments(bytes, Kind::Convert).is_err(), "{bytes}");
    }
    for bytes in [
        "result.expect(message())",
        "result.expect({ effect(); \"message\" })",
        "result.expect(SAVED)",
        "result.unwrap()",
        "result.expect::<u64>(\"message\")",
    ] {
        assert!(source_arguments(bytes, Kind::Expect).is_err(), "{bytes}");
    }
}

#[cfg(target_pointer_width = "64")]
#[test]
fn independent_64_bit_boundary_values_and_real_error_semantics() {
    let examples = [
        (0usize, 0u64),
        (1, 1),
        (4_294_967_295, 4_294_967_295),
        (4_294_967_296, 4_294_967_296),
        (9_223_372_036_854_775_807, 9_223_372_036_854_775_807),
        (9_223_372_036_854_775_808, 9_223_372_036_854_775_808),
        (usize::MAX, 18_446_744_073_709_551_615),
    ];
    for (input, expected) in examples {
        let actual: Result<u64, std::num::TryFromIntError> = u64::try_from(input);
        assert_eq!(actual, Ok(expected));
        assert_eq!(actual.expect("usize overflowed u64"), expected);
    }
    // The same error type does not make narrower conversions or arbitrary Err total.
    assert!(u16::try_from(65_536usize).is_err());
    let arbitrary = u16::try_from(65_536usize).map(u64::from);
    assert!(std::panic::catch_unwind(|| arbitrary.expect("real error")).is_err());
}

fn spelling(value: &mut Value, root: &str) {
    match value {
        Value::String(text) => {
            *text = text.replace("std::", root).replace("core::", root);
        }
        Value::Array(items) => items.iter_mut().for_each(|value| spelling(value, root)),
        Value::Object(fields) => fields.values_mut().for_each(|value| spelling(value, root)),
        _ => {}
    }
}

#[test]
fn std_and_no_std_displays_preserve_the_same_concrete_core_implementation() {
    let registry = registry();
    for root in ["std::", "core::"] {
        for kind in [
            Kind::Convert,
            Kind::Expect,
            Kind::ConvertU32Usize,
            Kind::ExpectUsize,
        ] {
            let mut observation = call(kind);
            spelling(&mut observation, root);
            assert!(selection::relationship(&observation, kind, &registry).is_ok());
            // Re-export spelling never grants authority to a different crate.
            observation["selected"]["crate"] = json!("std");
            assert!(selection::relationship(&observation, kind, &registry).is_err());
        }
    }
}

#[test]
fn a_conversion_cannot_swap_self_and_input_or_reuse_a_lossy_selected_body() {
    let registry = registry();
    let mut reversed = call(Kind::Convert);
    reversed["requested_arguments"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert!(selection::relationship(&reversed, Kind::Convert, &registry).is_err());
    for selected in [
        definition(CONVERT, "try-trait"),
        definition(
            "std::convert::num::ptr_try_from_impls::<impl std::convert::TryFrom<usize> for u16>::try_from",
            "exact-impl",
        ),
    ] {
        let mut forged = call(Kind::Convert);
        forged["selected"] = selected;
        assert!(selection::relationship(&forged, Kind::Convert, &registry).is_err());
    }
}

#[test]
fn real_error_ownership_cannot_come_from_the_display_name_or_unknown_substitution() {
    let registry = registry();
    for (pointer, value) in [
        ("/identity/definition/stable_crate_id", json!("other-core")),
        (
            "/identity/definition/definition_identity",
            json!("other-error"),
        ),
        ("/identity/definition/local", json!(true)),
        ("/identity/arguments", json!([builtin("u64")])),
        ("/identity_status", json!("unknown")),
        (
            "/identity",
            json!({"kind":"builtin", "name":"TryFromIntError"}),
        ),
    ] {
        let mut forged = call(Kind::Expect);
        for arguments in ["requested_arguments", "selected_arguments"] {
            *forged[arguments][1].pointer_mut(pointer).unwrap() = value.clone();
        }
        assert!(
            selection::relationship(&forged, Kind::Expect, &registry).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn overlapping_hir_and_source_claims_cannot_hide_a_second_selected_instance() {
    for pointer in ["/hir/local", "/span/end"] {
        let mut other = call(Kind::Convert);
        *other.pointer_mut(pointer).unwrap() = json!(99);
        let mut forged = inventory();
        forged["calls"].as_array_mut().unwrap().push(other);
        assert!(selection::site(&forged, &node()).is_err(), "{pointer}");
    }
    let mut external = inventory();
    external["calls"][0]["compiled_dependency"] = json!({
        "crate_name":"core", "stable_crate_id":"selected-core"
    });
    assert!(selection::site(&external, &node()).is_err());
}
