use super::*;

#[test]
fn range_token_is_canonical_native_width_and_root_only() {
    let text = r#"{"kind":"range_usize"}"#;
    let token = parse(text).unwrap();
    assert_eq!(token, TypeToken::RangeUsize);
    assert!(token.native_width());
    assert_eq!(token.text().unwrap(), text);
    for composite in [
        TypeToken::Tuple(vec![token.clone()]),
        TypeToken::Shared(Box::new(token.clone())),
        TypeToken::Array {
            element: Box::new(token.clone()),
            length: 1,
        },
    ] {
        assert!(parse(&composite.text().unwrap()).is_err());
    }
    assert!(parse(r#"{"kind":"range_usize","value":"usize"}"#).is_err());
}

#[test]
fn range_token_requires_new_capability() {
    use crate::extraction_profile::ExtractionProfile;
    let text = r#"{"kind":"range_usize"}"#;
    assert!(parse_for(text, ExtractionProfile::DataV2).is_err());
    assert!(parse_for(text, ExtractionProfile::DataV3).is_ok());
}

#[test]
fn range_to_is_observational_and_has_no_standalone_profile_permission() {
    use crate::extraction_profile::ExtractionProfile::{DataV2, DataV3};
    let text = r#"{"kind":"range_to_usize"}"#;
    let token = parse(text).unwrap();
    assert_eq!(token, TypeToken::RangeToUsize);
    assert!(token.native_width());
    assert_eq!(token.text().unwrap(), text);
    assert!(parse_for(text, DataV2).is_err());
    assert!(parse_for(text, DataV3).is_err());
    assert!(parse(&TypeToken::Tuple(vec![token]).text().unwrap()).is_err());
}

#[test]
fn borrowed_byte_slice_option_observation_is_exact_and_capability_gated() {
    use crate::extraction_profile::ExtractionProfile::{DataV2, DataV3};
    let token = TypeToken::Option(Box::new(TypeToken::Shared(Box::new(TypeToken::Slice(
        Box::new(TypeToken::Scalar("u8".into())),
    )))));
    let text = token.text().unwrap();
    assert_eq!(parse(&text).unwrap(), token);
    assert!(parse_for(&text, DataV2).is_err());
    assert_eq!(
        parse_for(&text, DataV3).is_ok(),
        DataV3.supports("rust.checked-borrowed-result-flow")
    );
    for payload in [
        TypeToken::Slice(Box::new(TypeToken::Scalar("u8".into()))),
        TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
            "u16".into(),
        ))))),
    ] {
        let token = TypeToken::Option(Box::new(payload));
        assert!(parse(&token.text().unwrap()).is_err());
    }
}

#[test]
fn non_copy_suffix_carrier_grants_no_standalone_or_nested_value_permission() {
    use crate::extraction_profile::ExtractionProfile::{DataV2, DataV3};
    let text = r#"{"kind":"range_from_usize"}"#;
    let token = parse(text).unwrap();
    assert_eq!(token, TypeToken::RangeFromUsize);
    assert!(token.native_width());
    assert_eq!(token.text().unwrap(), text);
    for profile in [DataV2, DataV3] {
        assert!(parse_for(text, profile).is_err());
    }
    assert!(parse(&TypeToken::Tuple(vec![token]).text().unwrap()).is_err());
}
