//! Never errors stay distinct from opaque inhabited errors and ordinary values.
use super::*;

fn carrier(ok: TypeTree) -> TypeTree {
    TypeTree::NeverConversionResult { ok: Box::new(ok) }
}

#[test]
fn never_carriers_preserve_width_and_deny_standalone_or_nested_value_use() {
    use crate::extraction_profile::ExtractionProfile;
    for payload in [
        TypeTree::Scalar("usize".into()),
        TypeTree::Tuple(vec![TypeTree::Scalar("usize".into()); 2]),
    ] {
        let tree = carrier(payload);
        let text = token(&tree).unwrap();
        assert_eq!(parse(&text).unwrap(), TypeToken::from_tree(&tree));
        assert!(parse(&text).unwrap().native_width());
        for observed in [tree.clone(), TypeTree::Tuple(vec![tree])] {
            for profile in [ExtractionProfile::DataV2, ExtractionProfile::DataV3] {
                assert!(parse_for(&token(&observed).unwrap(), profile).is_err());
            }
        }
    }
    let never = token(&TypeTree::NeverError).unwrap();
    assert_eq!(parse(&never).unwrap(), TypeToken::NeverError);
    assert!(parse_for(&never, ExtractionProfile::DataV3).is_err());
    assert_ne!(never, "unit");
    assert_ne!(
        token(&carrier(TypeTree::Scalar("usize".into()))).unwrap(),
        token(&TypeTree::IntConversionResult("usize".into())).unwrap()
    );
}

#[test]
fn never_carriers_reject_inhabited_and_adjacent_payload_smuggling() {
    for payload in [
        TypeTree::NeverError,
        TypeTree::Scalar("unit".into()),
        TypeTree::Scalar("u16".into()),
        TypeTree::Tuple(vec![]),
        TypeTree::Tuple(vec![TypeTree::Scalar("usize".into())]),
        TypeTree::Tuple(vec![TypeTree::Scalar("usize".into()); 3]),
        TypeTree::Tuple(vec![
            TypeTree::Scalar("u16".into()),
            TypeTree::Scalar("usize".into()),
        ]),
        carrier(TypeTree::Scalar("usize".into())),
    ] {
        let tree = carrier(payload);
        assert!(crate::concrete_types::validate(&tree).is_err());
        assert!(parse(&serde_json::to_string(&TypeToken::from_tree(&tree)).unwrap()).is_err());
    }
}
