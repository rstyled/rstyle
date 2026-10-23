use super::*;
use crate::data_types::{self, TypeToken};

struct Unresolved;
impl source_proof::PathResolution for Unresolved {
    fn resolve(&self, _: &syn::TypePath) -> Result<TypeTree, String> {
        Err("independent test has no alias authority".into())
    }
}

#[test]
fn mutable_usize_has_distinct_canonical_descriptor_identity() {
    let tree = TypeTree::MutableScalar("usize".into());
    validate(&tree).unwrap();
    let token = data_types::token(&tree).unwrap();
    assert_eq!(token, r#"{"kind":"mutable_scalar","value":"usize"}"#);
    assert_eq!(
        data_types::parse(&token).unwrap(),
        TypeToken::MutableScalar("usize".into())
    );
    assert!(data_types::parse(&token).unwrap().native_width());
    assert_ne!(token, "usize");
    assert_ne!(
        token,
        data_types::token(&TypeTree::Shared(Box::new(TypeTree::Scalar(
            "usize".into()
        ))))
        .unwrap()
    );
}

#[test]
fn mutable_descriptors_require_exact_usize_at_root() {
    for scalar in ["u8", "u64", "isize", "unit", "bogus"] {
        assert!(validate(&TypeTree::MutableScalar(scalar.into())).is_err());
        assert!(
            data_types::parse(&format!(
                r#"{{"kind":"mutable_scalar","value":"{scalar}"}}"#
            ))
            .is_err()
        );
    }
    let tree = TypeTree::MutableScalar("usize".into());
    for nested in [
        TypeTree::Tuple(vec![tree.clone()]),
        TypeTree::Shared(Box::new(tree.clone())),
        TypeTree::Slice(Box::new(tree.clone())),
        TypeTree::Array {
            element: Box::new(tree),
            length: 1,
        },
    ] {
        assert!(validate(&nested).is_err());
        let serialized = TypeToken::from_tree(&nested).text().unwrap();
        assert!(data_types::parse(&serialized).is_err());
    }
}

#[test]
fn mutable_source_proof_keeps_mutability_and_referent_distinct() {
    let tree = TypeTree::MutableScalar("usize".into());
    for source in ["&mut usize", "&'a mut usize", "(&mut usize)"] {
        source_proof::verify_top_spelling(source, &tree, &Unresolved).unwrap();
    }
    for source in [
        "usize",
        "&usize",
        "&mut u8",
        "&mut &mut usize",
        "&mut Alias",
        "*mut usize",
    ] {
        assert!(
            source_proof::verify_top_spelling(source, &tree, &Unresolved).is_err(),
            "{source}"
        );
    }
    let shared = TypeTree::Shared(Box::new(TypeTree::Scalar("usize".into())));
    assert!(source_proof::verify_top_spelling("&mut usize", &shared, &Unresolved).is_err());
}

#[test]
fn historical_profiles_and_unsupported_observations_gain_no_permission() {
    let token = data_types::token(&TypeTree::MutableScalar("usize".into())).unwrap();
    for profile in [
        crate::extraction_profile::ExtractionProfile::R0,
        crate::extraction_profile::ExtractionProfile::DataV1,
        crate::extraction_profile::ExtractionProfile::DataV2,
    ] {
        assert!(data_types::parse_for(&token, profile).is_err());
    }
    let legacy = r#"{"status":"Unsupported","reason":"mutable storage unsupported"}"#;
    let observation: Observation = serde_json::from_str(legacy).unwrap();
    assert!(matches!(&observation, Observation::Unsupported { .. }));
    assert_eq!(serde_json::to_string(&observation).unwrap(), legacy);
}

#[test]
fn historical_data392_documents_do_not_gain_mutable_memory_capability() {
    let old = crate::ProfileId {
        id: crate::extraction_profile::DATA_ID.into(),
        version: "3.92.0".into(),
    };
    assert!(!crate::extraction_profile::policy_supports(
        &old,
        crate::mutable_scalars::CAPABILITY,
    ));
    assert!(
        crate::extraction_profile::ExtractionProfile::DataV3
            .supports(crate::mutable_scalars::CAPABILITY,)
    );
}
