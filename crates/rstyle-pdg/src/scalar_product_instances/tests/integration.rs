use super::{ORIGINAL, observation};
use crate::{
    concrete_types::{self, TypeTree},
    data_origins,
    data_types::{self, TypeToken},
    extraction_profile::ExtractionProfile,
    scalar_product_instances::Argument,
};

fn instance(argument: Argument) -> TypeTree {
    TypeTree::ScalarProductInstance(Box::new(observation(ORIGINAL, argument)))
}

#[test]
fn instance_origins_retain_the_original_generic_declaration() {
    use concrete_types::{IDENTITY, InterfaceObservations, Observation};
    let tree = instance(Argument::U16);
    let interface = InterfaceObservations {
        identity: IDENTITY.into(),
        parameters: Vec::new(),
        result: Observation::Captured {
            ty: TypeTree::Option(Box::new(tree.clone())),
        },
    };
    let declarations = crate::declaration_origins::declarations(&interface).unwrap();
    let (nominal, source) = tree.nominal_parts().unwrap();
    assert_eq!(declarations.len(), 1);
    assert_eq!(
        declarations.get(&nominal.definition_path),
        Some(&(nominal, source.as_str()))
    );
    assert_eq!(source.as_str(), ORIGINAL);
    assert!(source.contains(" T)"));
    assert!(
        crate::declaration_origins::verify(
            &interface,
            &crate::declaration_origins::Origins {
                identity: crate::declaration_origins::IDENTITY.into(),
                declarations: Vec::new(),
                generated: Vec::new(),
            }
        )
        .is_err()
    );
}

fn nested_tokens() -> Vec<TypeToken> {
    let value = TypeToken::from_tree(&instance(Argument::U8));
    vec![
        value.clone(),
        TypeToken::Option(Box::new(value.clone())),
        TypeToken::Tuple(vec![TypeToken::Scalar("bool".into()), value.clone()]),
        TypeToken::Array {
            element: Box::new(value.clone()),
            length: 2,
        },
        TypeToken::Slice(Box::new(value)),
    ]
}

#[test]
fn same_declaration_keeps_distinct_instance_tokens_and_original_generic_source() {
    let byte_tree = instance(Argument::U8);
    let short_tree = instance(Argument::U16);
    let byte = data_types::token(&byte_tree).unwrap();
    let short = data_types::token(&short_tree).unwrap();
    assert_ne!(byte, short);
    let TypeToken::ScalarProductInstance {
        nominal: byte_nominal,
        argument: byte_argument,
        fields: byte_fields,
    } = data_types::parse(&byte).unwrap()
    else {
        panic!("Original product instance was erased");
    };
    let TypeToken::ScalarProductInstance {
        nominal: short_nominal,
        argument: short_argument,
        fields: short_fields,
    } = data_types::parse(&short).unwrap()
    else {
        panic!("Original product instance was erased");
    };
    assert_eq!(byte_nominal, short_nominal);
    assert_eq!(byte_argument, Argument::U8);
    assert_eq!(short_argument, Argument::U16);
    assert_eq!(byte_fields[0], short_fields[0]);
    assert_eq!(byte_fields[1], ("1".into(), TypeToken::Scalar("u8".into())));
    assert_eq!(
        short_fields[1],
        ("1".into(), TypeToken::Scalar("u16".into()))
    );
}

#[test]
fn raw_tree_and_option_encoding_roundtrip_without_value_authority() {
    for argument in [Argument::U8, Argument::U16] {
        let direct = instance(argument);
        for tree in [direct.clone(), TypeTree::Option(Box::new(direct))] {
            concrete_types::validate(&tree).unwrap();
            let wire = serde_json::to_string(&tree).unwrap();
            let restored: TypeTree = serde_json::from_str(&wire).unwrap();
            assert_eq!(restored, tree);
            let text = data_types::token(&restored).unwrap();
            let token = data_types::parse(&text).unwrap();
            assert_eq!(token, TypeToken::from_tree(&tree));
            assert_eq!(token.text().unwrap(), text);
        }
    }
}

#[test]
fn unsupported_nested_instances_never_appear_reference_free_or_gain_flat_origins() {
    for token in nested_tokens() {
        assert!(data_origins::shared(&token), "{token:?}");
        assert!(
            data_origins::paths(&token, Vec::new()).is_err(),
            "{token:?}"
        );
    }
    let value = TypeToken::from_tree(&instance(Argument::U16));
    let result = TypeToken::Result {
        ok: Box::new(value),
        err: Box::new(TypeToken::Scalar("u8".into())),
    };
    assert!(data_origins::shared(&result));
    assert!(data_origins::paths(&result, Vec::new()).is_err());
}

#[test]
fn every_existing_profile_rejects_unproved_direct_and_nested_instance_values() {
    let profiles = [
        ExtractionProfile::R0,
        ExtractionProfile::ConstV1,
        ExtractionProfile::CastsV1,
        ExtractionProfile::MatchV1,
        ExtractionProfile::GuardsV1,
        ExtractionProfile::ConditionalV1,
        ExtractionProfile::LoopsV1,
        ExtractionProfile::CallsV1,
        ExtractionProfile::HelpersV1,
        ExtractionProfile::DataV1,
        ExtractionProfile::DataV2,
        ExtractionProfile::DataV3,
    ];
    for token in nested_tokens() {
        let text = token.text().unwrap();
        assert_eq!(data_types::parse(&text).unwrap(), token);
        for profile in profiles {
            assert!(
                data_types::parse_for(&text, profile).is_err(),
                "{profile:?}: {text}"
            );
        }
    }
}

#[test]
fn token_substitution_and_complete_roster_are_checked_without_original_source() {
    let original = TypeToken::from_tree(&instance(Argument::U8));
    let mut changed_argument = original.clone();
    let TypeToken::ScalarProductInstance { argument, .. } = &mut changed_argument else {
        unreachable!()
    };
    *argument = Argument::U16;
    let mut changed_fields = Vec::new();
    for mutation in 0..6 {
        let mut changed = original.clone();
        let TypeToken::ScalarProductInstance { fields, .. } = &mut changed else {
            unreachable!()
        };
        match mutation {
            0 => {
                fields[1].1 = TypeToken::Scalar("u32".into());
            }
            1 => {
                fields[0].1 = TypeToken::Shared(Box::new(TypeToken::Scalar("u8".into())));
            }
            2 => {
                fields.swap(0, 1);
            }
            3 => {
                fields[1].0 = "0".into();
            }
            4 => {
                fields.pop();
            }
            _ => {
                fields.push(fields[1].clone());
            }
        }
        changed_fields.push(changed);
    }
    for changed in std::iter::once(changed_argument).chain(changed_fields) {
        assert!(
            data_types::parse(&changed.text().unwrap()).is_err(),
            "{changed:?}"
        );
    }
    assert!(data_types::parse(&original.text().unwrap()).is_ok());
}
