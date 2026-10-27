use super::*;
use crate::{
    HirFnFacts,
    concrete_types::{InterfaceObservations, Observation, TypeTree},
    data_types::TypeToken,
};

fn originals() -> Vec<HirFnFacts> {
    let mut facts: Vec<HirFnFacts> =
        serde_json::from_str(include_str!("../source/tests/originals.json")).unwrap();
    for original in &mut facts {
        original
            .build
            .frontend_options
            .insert_data(InterfaceObservations {
                identity: crate::concrete_types::IDENTITY.into(),
                parameters: vec![Observation::Captured {
                    ty: TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(TypeTree::Scalar(
                        "u8".into(),
                    ))))),
                }],
                result: Observation::Unsupported {
                    reason:
                        "Location-only test; product result is checked independently at integration"
                            .into(),
                },
            });
    }
    facts
}

#[test]
fn regions_preserve_independently_expected_element_offsets() {
    for (index, facts) in originals().iter().enumerate() {
        let checked = super::super::semantics::verify(&facts.body.children[0], facts).unwrap();
        let count = (index + 1) as u8;
        for position in 0..count {
            let origin = Region::Element { position }
                .checked_origin(&checked)
                .unwrap();
            assert_eq!(origin.parameter, 0);
            assert_eq!(
                origin.path,
                [
                    Projection::SharedReferent,
                    Projection::PrefixElement { position }
                ]
            );
            let values = crate::field_locations::derived::from_origins(
                TypeToken::Scalar("u8".into()),
                facts,
                vec![origin],
            )
            .unwrap();
            assert_eq!(values[0].value, TypeTree::Scalar("u8".into()));
        }
        assert!(
            Region::Element { position: count }
                .checked_origin(&checked)
                .is_err()
        );
    }
}

#[test]
fn regions_preserve_independently_expected_one_past_suffix_offsets() {
    for (index, facts) in originals().iter().enumerate() {
        let checked = super::super::semantics::verify(&facts.body.children[0], facts).unwrap();
        let count = (index + 1) as u8;
        let origin = Region::Suffix { start: count }
            .checked_origin(&checked)
            .unwrap();
        assert_eq!(
            origin.path,
            [
                Projection::SharedReferent,
                Projection::PrefixSuffix { start: count }
            ]
        );
        let values = crate::field_locations::derived::from_origins(
            TypeToken::Slice(Box::new(TypeToken::Scalar("u8".into()))),
            facts,
            vec![origin],
        )
        .unwrap();
        assert_eq!(
            values[0].value,
            TypeTree::Slice(Box::new(TypeTree::Scalar("u8".into())))
        );
        assert!(
            Region::Element { position: count }
                .checked_origin(&checked)
                .is_err()
        );
        for start in [0, count + 1] {
            assert!(Region::Suffix { start }.checked_origin(&checked).is_err());
        }
    }
}

#[test]
fn selector_and_body_share_one_location_but_distinct_positions_never_collapse() {
    for facts in originals() {
        let checked = super::super::semantics::verify(&facts.body.children[0], &facts).unwrap();
        let mut context = LocationContext::new(&facts);
        let mut ids = std::collections::BTreeSet::new();
        for position in 0..checked.consumed() {
            let region = Region::Element { position };
            let (selector, locations) = region_reads(&mut context, &checked, &region).unwrap();
            let (body, _) = region_reads(&mut context, &checked, &region).unwrap();
            assert_eq!(selector, body);
            assert_eq!(selector.len(), 1);
            assert_eq!(selector[0].operand_position, Some(0));
            assert!(selector[0].symbol_id.is_none());
            assert!(ids.insert(selector[0].location_id.clone().unwrap()));
            let location = locations.values().next().unwrap();
            assert_eq!(location.abstraction, "rstyle.formal-prefix-element/1");
            let record = location.extensions.values().next().unwrap();
            assert_eq!(record["parameter"], 0);
            assert_eq!(
                record["path"],
                serde_json::to_value([
                    Projection::SharedReferent,
                    Projection::PrefixElement { position }
                ])
                .unwrap()
            );
        }
        assert!(
            region_reads(
                &mut context,
                &checked,
                &Region::Suffix {
                    start: checked.consumed()
                }
            )
            .is_err()
        );
    }
}

#[test]
fn projection_interpretation_rejects_out_of_contract_offsets() {
    let facts = originals().remove(0);
    for projection in [
        Projection::PrefixElement { position: 3 },
        Projection::PrefixSuffix { start: 0 },
        Projection::PrefixSuffix { start: 4 },
    ] {
        let origin = Origin {
            parameter: 0,
            path: vec![Projection::SharedReferent, projection],
        };
        assert!(
            crate::field_locations::derived::from_origins(
                TypeToken::Scalar("u8".into()),
                &facts,
                vec![origin]
            )
            .is_err()
        );
    }
}

#[test]
fn projection_interpretation_rejects_wrong_storage() {
    let mut facts = originals().remove(0);
    let mut interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()
        .unwrap()
        .into_owned();
    for storage in [
        TypeTree::Str,
        TypeTree::Slice(Box::new(TypeTree::Scalar("u16".into()))),
        TypeTree::Array {
            element: Box::new(TypeTree::Scalar("u8".into())),
            length: 3,
        },
    ] {
        interface.parameters[0] = Observation::Captured {
            ty: TypeTree::Shared(Box::new(storage)),
        };
        facts.build.frontend_options.insert(
            crate::concrete_types::KEY.into(),
            serde_json::to_value(&interface).unwrap(),
        );
        let origin = Origin {
            parameter: 0,
            path: vec![
                Projection::SharedReferent,
                Projection::PrefixElement { position: 0 },
            ],
        };
        assert!(
            crate::field_locations::derived::from_origins(
                TypeToken::Scalar("u8".into()),
                &facts,
                vec![origin]
            )
            .is_err()
        );
    }
}
