use super::*;

fn mapped(payload: TypeTree) -> TypeTree {
    TypeTree::MappedIntConversionResult {
        ok: Box::new(payload),
    }
}

fn tuple(names: &[&str]) -> TypeTree {
    TypeTree::Tuple(
        names
            .iter()
            .map(|name| TypeTree::Scalar((*name).into()))
            .collect(),
    )
}

#[test]
fn mapped_conversion_preserves_real_tuple_and_native_width() {
    let tree = mapped(tuple(&["usize", "usize"]));
    crate::concrete_types::validate(&tree).unwrap();
    let text = token(&tree).unwrap();
    assert_eq!(
        text,
        r#"{"kind":"mapped_int_conversion_result","value":{"ok":{"kind":"tuple","value":[{"kind":"scalar","value":"usize"},{"kind":"scalar","value":"usize"}]}}}"#
    );
    let parsed = parse(&text).unwrap();
    assert_eq!(parsed, TypeToken::from_tree(&tree));
    assert!(parsed.native_width());
}

#[test]
fn mapped_conversion_rejects_adjacent_and_smuggled_payloads() {
    for payload in [
        tuple(&[]),
        tuple(&["usize"]),
        tuple(&["usize", "usize", "usize"]),
        tuple(&["u32", "usize"]),
        tuple(&["usize", "u32"]),
        TypeTree::Scalar("(usize, usize)".into()),
        TypeTree::Shared(Box::new(tuple(&["usize", "usize"]))),
        mapped(tuple(&["usize", "usize"])),
    ] {
        let tree = mapped(payload);
        assert!(crate::concrete_types::validate(&tree).is_err());
        let token = TypeToken::from_tree(&tree);
        assert!(parse(&serde_json::to_string(&token).unwrap()).is_err());
    }
}

#[test]
fn scalar_conversion_wire_bytes_stay_unchanged() {
    let scalar = TypeTree::IntConversionResult("usize".into());
    assert_eq!(
        serde_json::to_string(&scalar).unwrap(),
        r#"{"kind":"IntConversionResult","evidence":"usize"}"#
    );
    assert_eq!(
        token(&scalar).unwrap(),
        r#"{"kind":"int_conversion_result","value":"usize"}"#
    );
    assert!(
        crate::concrete_types::validate(&TypeTree::IntConversionResult("(usize,usize)".into()))
            .is_err()
    );
}

#[test]
fn mapped_conversion_unknown_fields_and_missing_payload_are_rejected() {
    for value in [
        r#"{"kind":"mapped_int_conversion_result","value":{}}"#,
        r#"{"kind":"mapped_int_conversion_result","value":{"ok":{"kind":"tuple","value":[]},"drop_free":true}}"#,
    ] {
        assert!(parse(value).is_err());
    }
}

#[test]
fn mapped_observations_never_grant_value_permission_to_any_profile() {
    use crate::extraction_profile::ExtractionProfile;
    let observed = mapped(tuple(&["usize", "usize"]));
    for tree in [
        observed.clone(),
        TypeTree::Tuple(vec![observed.clone()]),
        TypeTree::Shared(Box::new(observed)),
    ] {
        let text = token(&tree).unwrap();
        assert!(parse(&text).is_ok());
        for profile in [ExtractionProfile::DataV2, ExtractionProfile::DataV3] {
            let error = parse_for(&text, profile).unwrap_err();
            assert!(error.contains("callback value evidence"), "{error}");
        }
    }
}
