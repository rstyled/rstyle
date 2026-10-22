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
