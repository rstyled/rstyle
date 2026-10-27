//! Independent complete-product expectations; none of these mint checked authority.
use super::*;
use crate::concrete_types::VariantShape;
use serde_json::Value;

fn canonical() -> HirFnFacts {
    let operations: Value =
        serde_json::from_str(include_str!("../operations/fixtures.json")).unwrap();
    let bodies: Value = serde_json::from_str(include_str!("../body/fixtures.json")).unwrap();
    let mut facts: HirFnFacts = serde_json::from_value(operations[0]["canonical"].clone()).unwrap();
    facts.signature = serde_json::from_value(bodies[0]["canonical_signature"].clone()).unwrap();
    facts.body = serde_json::from_value(bodies[0]["canonical_body"].clone()).unwrap();
    facts
}

#[test]
fn complete_result_keeps_the_nonselected_pattern_payload_and_exact_subproducts() {
    let products = Products::checked(&canonical()).unwrap();
    assert!(products.permits(products.input()));
    assert!(products.permits(products.span()));
    assert!(products.permits(products.anchored()));
    let TypeToken::MixedEnum { variants, .. } = products.anchored() else {
        panic!("complete mixed enum");
    };
    assert_eq!(
        variants
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["No", "Yes", "Pattern"]
    );
    let VariantShape::SingleTuple { field: pattern } = &variants[2].1 else {
        panic!("Pattern payload retained");
    };
    assert!(products.permits(pattern));
    let TypeToken::Struct { fields, .. } = pattern else {
        panic!("PatternID product");
    };
    assert!(products.permits(&fields[0].1));
    assert!(!products.permits(&TypeToken::Shared(Box::new(products.input().clone()))));
}

#[test]
fn exact_type_permit_rejects_foreign_nominals_incomplete_payloads_and_fieldless_reinterpretation() {
    let products = Products::checked(&canonical()).unwrap();
    let mut foreign = products.input().clone();
    let TypeToken::Struct { nominal, .. } = &mut foreign else {
        unreachable!()
    };
    nominal.definition_path = "another::Input".into();
    assert!(!products.permits(&foreign));
    let TypeToken::MixedEnum { nominal, .. } = products.anchored() else {
        unreachable!()
    };
    let fieldless = TypeToken::FieldlessEnum {
        nominal: nominal.clone(),
        variants: vec![("No".into(), 0), ("Yes".into(), 1)],
    };
    assert!(!products.permits(&fieldless));
    assert!(shape::verify(&fieldless).is_err());
}

#[test]
fn missing_or_changed_nonselected_payloads_fail_the_complete_shape_contract() {
    let products = Products::checked(&canonical()).unwrap();
    for mode in ["omitted", "reordered", "unit", "payload", "index"] {
        let mut ty = products.input().clone();
        let TypeToken::Struct { fields, .. } = &mut ty else {
            unreachable!()
        };
        let TypeToken::MixedEnum { variants, .. } = &mut fields[2].1 else {
            unreachable!()
        };
        match mode {
            "omitted" => {
                variants.pop();
            }
            "reordered" => variants.swap(0, 1),
            "unit" => variants[2].1 = VariantShape::Unit {},
            "payload" => {
                variants[2].1 = VariantShape::SingleTuple {
                    field: TypeToken::Scalar("u32".into()),
                }
            }
            "index" => {
                let VariantShape::SingleTuple {
                    field: TypeToken::Struct { fields, .. },
                } = &mut variants[2].1
                else {
                    unreachable!()
                };
                let TypeToken::Struct { fields, .. } = &mut fields[0].1 else {
                    unreachable!()
                };
                fields[0].1 = TypeToken::Scalar("u64".into());
            }
            _ => unreachable!(),
        }
        assert!(shape::verify(&ty).is_err(), "{mode}");
        assert!(!products.permits(&ty), "{mode}");
    }
}

#[test]
fn exact_site_check_rejects_copied_hir_with_changed_source_type_or_effects() {
    let facts = canonical();
    let original = &facts.body.children[1].children[2];
    exact(original, original).unwrap();
    for mode in ["source", "type", "effect", "child"] {
        let mut changed = original.clone();
        match mode {
            "source" => changed.snippet = "Anchored::Yes".into(),
            "type" => changed.ty.as_mut().unwrap().normalized = Some("u8".into()),
            "effect" => changed.is_unsafe_block = true,
            "child" => changed.children.push(original.clone()),
            _ => unreachable!(),
        }
        assert!(exact(&changed, original).is_err(), "{mode}");
    }
}

#[test]
fn complete_profile_and_type_data_cannot_replace_a_private_instance_owner() {
    let facts = canonical();
    let result = &facts.body.children[1];
    let variant = &result.children[2];
    assert!(!struct_candidate(result, &facts));
    assert!(!unit_candidate(variant, &facts));
    assert!(slots(result, &facts).is_err());
    assert!(unit(variant, &facts).is_err());
}
