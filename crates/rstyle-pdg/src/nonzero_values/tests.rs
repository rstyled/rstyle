use super::*;

fn facts() -> crate::HirFnFacts {
    let fixture: Json = serde_json::from_str(include_str!(
        "../reference_instances/operations/fixtures.json"
    ))
    .unwrap();
    serde_json::from_value(fixture[0]["canonical"].clone()).unwrap()
}

#[test]
fn every_u8_u16_input_obeys_independent_zero_and_exact_payload_expectations() {
    let facts = facts();

    for scalar in [Scalar::U8, Scalar::U16] {
        let observation = authority::observation(scalar);
        let checked = verify(&observation, &facts).unwrap();

        for input in 0..=scalar.maximum() {
            let value = checked.construct(input).unwrap();
            assert_eq!(value.is_some(), input > 0);
            if let Some(value) = value {
                assert_eq!((value.scalar(), value.get()), (scalar, input));
            }
        }
        assert!(checked.construct(scalar.maximum() + 1).is_err());
        assert!(checked.construct(u64::MAX).is_err());
    }
}

#[test]
fn exact_type_and_safe_constructor_are_required() {
    let facts = facts();

    for scalar in [Scalar::U8, Scalar::U16] {
        let observation = authority::observation(scalar);
        assert_eq!(
            observe_type(&structural(&observation), &facts).unwrap(),
            observation
        );

        let callable = constructor(&observation);
        assert_eq!(verify_new(&callable, &facts).unwrap(), observation);
        for pointer in [
            "/definition/definition_identity",
            "/definition/stable_crate_id",
            "/definition/crate",
            "/definition/path",
            "/arguments/0/name",
            "/signature/output/definition/definition_identity",
            "/signature/output/arguments/0/definition/definition_identity",
            "/signature/output/arguments/0/arguments/0/name",
            "/signature/inputs/0/name",
            "/signature/abi",
            "/signature/safety",
        ] {
            let mut changed = callable.clone();
            *changed.pointer_mut(pointer).unwrap() = json!("unreviewed");
            assert!(verify_new(&changed, &facts).is_err(), "{pointer}");
        }
        for (pointer, replacement) in [
            ("/definition/local", json!(true)),
            ("/signature/bound_variables", json!(1)),
            ("/signature/variadic", json!(true)),
            ("/arguments", json!([])),
            ("/signature/inputs", json!([])),
        ] {
            let mut changed = callable.clone();
            *changed.pointer_mut(pointer).unwrap() = replacement;
            assert!(verify_new(&changed, &facts).is_err(), "{pointer}");
        }
    }
}

#[test]
fn lookalike_nominals_and_nonreviewed_builds_are_rejected() {
    let facts = facts();
    let observation = authority::observation(Scalar::U8);
    let mut lookalike = observation.clone();
    lookalike.definition.local = true;
    assert!(lookalike.check_shape().is_err());
    assert!(verify(&lookalike, &facts).is_err());

    let mut changed = facts.clone();
    changed.build.target_triple.value = Some("x86_64-unknown-linux-gnu".into());
    assert!(verify(&observation, &changed).is_err());

    let mut changed = facts.clone();
    changed.build.rustc_commit.value = Some("unreviewed".into());
    assert!(verify(&observation, &changed).is_err());

    let mut changed = facts.clone();
    changed.build.frontend_options.insert(
        "invocation".into(),
        json!(["rustc", "--extern", "core=lookalike.rlib"]),
    );
    assert!(verify(&observation, &changed).is_err());
}

#[test]
fn native_copy_and_no_drop_contract_is_independent_of_niche_layout() {
    fn copy<T: Copy>() {}
    copy::<std::num::NonZeroU8>();
    copy::<std::num::NonZeroU16>();

    assert!(!std::mem::needs_drop::<std::num::NonZeroU8>());
    assert!(!std::mem::needs_drop::<std::num::NonZeroU16>());

    for input in 0..=u16::MAX {
        let expected = (input > 0).then_some(u64::from(input));
        assert_eq!(
            std::num::NonZeroU16::new(input).map(|v| u64::from(v.get())),
            expected
        );
        if let Ok(byte) = u8::try_from(input) {
            assert_eq!(
                std::num::NonZeroU8::new(byte).map(|v| u64::from(v.get())),
                expected
            );
        }
    }
}

#[test]
fn pinned_core_source_contains_reviewed_copy_storage_and_safe_constructor_contracts() {
    let output = std::process::Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let sysroot = String::from_utf8(output.stdout).unwrap();
    let source = std::fs::read_to_string(
        std::path::Path::new(sysroot.trim())
            .join("lib/rustlib/src/rust/library/core/src/num/nonzero.rs"),
    )
    .unwrap();

    let reviewed: Vec<String> = serde_json::from_str(include_str!("reviewed-source.json")).unwrap();
    assert_eq!(reviewed.len(), 4);
    for excerpt in reviewed {
        assert!(
            source.contains(&excerpt),
            "Reviewed core NonZero contract changed"
        );
    }
}

#[test]
fn structural_displays_extra_arguments_and_unknown_fields_cannot_supply_authority() {
    let facts = facts();
    let observation = authority::observation(Scalar::U16);
    for (pointer, replacement) in [
        (
            "/arguments",
            json!([builtin(Scalar::U16), builtin(Scalar::U8)]),
        ),
        ("/arguments/0/name", json!("NonZeroU16")),
        ("/kind", json!("builtin")),
        ("/definition/local", json!(true)),
    ] {
        let mut changed = structural(&observation);
        *changed.pointer_mut(pointer).unwrap() = replacement;
        assert!(observe_type(&changed, &facts).is_err(), "{pointer}");
    }
    let mut changed = structural(&observation);
    changed["needs_drop"] = json!(false);
    assert!(observe_type(&changed, &facts).is_err());
    let mut changed = constructor(&observation);
    changed["pure"] = json!(true);
    assert!(verify_new(&changed, &facts).is_err());
}

#[test]
fn concrete_nonzero_tokens_preserve_nominal_width_and_canonical_roundtrip() {
    use crate::{concrete_types::TypeTree, data_types};
    for scalar in [Scalar::U8, Scalar::U16] {
        let observation = authority::observation(scalar);
        let tree = TypeTree::NonZero(observation.clone());
        let token = data_types::TypeToken::from_tree(&tree);
        let text = data_types::token(&tree).unwrap();
        assert_eq!(data_types::parse(&text).unwrap(), token);
        assert_eq!(token.text().unwrap(), text);
        assert!(text.starts_with("{\"kind\":\"non_zero\",\"value\":{\"definition\":"));
        assert!(text.ends_with(&format!(",\"scalar\":\"{}\"}}}}", scalar.spelling())));
        assert_ne!(
            token,
            data_types::TypeToken::Scalar(scalar.spelling().into())
        );
        assert_ne!(text, scalar.spelling());

        let option = TypeTree::Option(Box::new(tree));
        let option_token = data_types::TypeToken::Option(Box::new(token));
        assert_eq!(data_types::TypeToken::from_tree(&option), option_token);
        let option_text = data_types::token(&option).unwrap();
        assert_eq!(data_types::parse(&option_text).unwrap(), option_token);
        assert_eq!(option_token.text().unwrap(), option_text);
    }
}

#[test]
fn concrete_token_shape_rejects_unreviewed_definitions_and_widths() {
    use crate::{concrete_types::TypeTree, data_types};
    let observation = authority::observation(Scalar::U8);
    let tree = TypeTree::NonZero(observation);
    let text = data_types::token(&tree).unwrap();
    for (pointer, replacement) in [
        ("/value/definition/definition_identity", json!("unreviewed")),
        ("/value/definition/crate", json!("application")),
        ("/value/definition/local", json!(true)),
        ("/value/scalar", json!("u32")),
        ("/value/scalar", json!("usize")),
    ] {
        let mut changed: Json = serde_json::from_str(&text).unwrap();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        let changed_text = serde_json::to_string(&changed).unwrap();
        assert!(data_types::parse(&changed_text).is_err(), "{pointer}");
        if let Ok(token) = serde_json::from_value::<data_types::TypeToken>(changed) {
            assert!(
                data_types::parse(&token.text().unwrap()).is_err(),
                "{pointer}"
            );
        }
    }
}

#[test]
fn raw_nonzero_shapes_grant_no_historical_or_current_profile_permission() {
    use crate::{concrete_types::TypeTree, data_types, extraction_profile::ExtractionProfile};
    for scalar in [Scalar::U8, Scalar::U16] {
        let tree = TypeTree::NonZero(authority::observation(scalar));
        for shape in [tree.clone(), TypeTree::Option(Box::new(tree))] {
            let text = data_types::token(&shape).unwrap();
            assert!(data_types::parse(&text).is_ok());
            for profile in [
                ExtractionProfile::R0,
                ExtractionProfile::DataV2,
                ExtractionProfile::DataV3,
            ] {
                assert!(data_types::parse_for(&text, profile).is_err());
            }
        }
    }
}

#[test]
fn reviewed_std_visible_paths_preserve_exact_core_nominal_authority() {
    let facts = facts();
    let paths = [
        "/definition/path",
        "/signature/output/definition/path",
        "/signature/output/arguments/0/definition/path",
    ];
    for scalar in [Scalar::U8, Scalar::U16] {
        let observation = authority::observation(scalar);
        for aliases in 0..8 {
            let mut callable = constructor(&observation);
            for (index, pointer) in paths.iter().enumerate() {
                if aliases & (1 << index) != 0 {
                    let slot = callable.pointer_mut(pointer).unwrap();
                    *slot = json!(slot.as_str().unwrap().replacen("core::", "std::", 1));
                }
            }
            assert_eq!(verify_new(&callable, &facts).unwrap(), observation);
        }

        let mut displayed = observation.clone();
        displayed.definition.path = "std::num::NonZero".into();
        displayed.check_shape().unwrap();
        verify(&displayed, &facts).unwrap();
        assert_eq!(displayed.clone().canonicalized().unwrap(), observation);
        assert_eq!(
            observe_type(&structural(&displayed), &facts).unwrap(),
            observation
        );
    }
}

#[test]
fn visible_aliases_cannot_replace_core_crate_or_definition_identity() {
    let facts = facts();
    let observation = authority::observation(Scalar::U8);
    for (pointer, replacement) in [
        (
            "/definition/path",
            json!("fakecrate::num::NonZero::<T>::new"),
        ),
        ("/definition/crate", json!("std")),
        ("/definition/definition_identity", json!("unreviewed")),
        (
            "/signature/output/definition/path",
            json!("fakecrate::option::Option"),
        ),
        (
            "/signature/output/arguments/0/definition/path",
            json!("fakecrate::num::NonZero"),
        ),
    ] {
        let mut callable = constructor(&observation);
        callable["definition"]["path"] = json!("std::num::NonZero::<T>::new");
        *callable.pointer_mut(pointer).unwrap() = replacement;
        assert!(verify_new(&callable, &facts).is_err(), "{pointer}");
    }
    let mut displayed = observation;
    displayed.definition.path = "fakecrate::num::NonZero".into();
    assert!(displayed.check_shape().is_err());
    assert!(displayed.clone().canonicalized().is_err());
    assert!(observe_type(&structural(&displayed), &facts).is_err());
}

#[test]
fn neighboring_nonzero_fields_retain_raw_shapes_without_source_or_admission_authority() {
    use crate::{
        concrete_types::{Field, Nominal, TypeTree},
        data_types,
        extraction_profile::ExtractionProfile,
    };
    for scalar in [Scalar::U8, Scalar::U16] {
        let declaration: crate::SourceText = format!(
            "struct Neighbor {{ value: Option<NonZero<{}>> }}",
            scalar.spelling()
        )
        .into();
        let tree = TypeTree::Struct {
            nominal: Nominal {
                source_identity: Some("source-only-neighbor".into()),
                package_name: "fixture".into(),
                package_version: "0.1.0".into(),
                manifest_digest: crate::source_identity::checksum(b"manifest"),
                definition_path: "Neighbor".into(),
                declaration_digest: declaration.checksum().into(),
            },
            source: declaration,
            fields: vec![Field {
                position: 0,
                name: "value".into(),
                declaration: format!("Option<NonZero<{}>>", scalar.spelling()).into(),
                ty: TypeTree::Option(Box::new(TypeTree::NonZero(authority::observation(scalar)))),
            }],
        };
        let token = data_types::token(&tree).unwrap();
        assert!(data_types::parse(&token).is_ok());
        for profile in [
            ExtractionProfile::R0,
            ExtractionProfile::DataV2,
            ExtractionProfile::DataV3,
        ] {
            assert!(data_types::parse_for(&token, profile).is_err());
        }
        assert!(crate::concrete_types::source_proof::verify(&tree, &UnresolvedNeighbor).is_err());
    }
}

struct UnresolvedNeighbor;
impl crate::concrete_types::source_proof::PathResolution for UnresolvedNeighbor {
    fn resolve(&self, _: &syn::TypePath) -> Result<crate::concrete_types::TypeTree, String> {
        Err("Observational shape supplies no lexical binding".into())
    }
}
