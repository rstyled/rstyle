use super::*;
use crate::concrete_types::{Field, MixedVariant, Nominal};

const SOURCE: &str = "#[derive(Debug)] #[non_exhaustive] enum Error { OutputTooSmall { expected: usize, actual: usize }, LiteralOutOfBounds, ExpectedAnotherByte, OffsetZero, OffsetOutOfBounds }";

fn tree() -> TypeTree {
    let fields = ["expected", "actual"]
        .into_iter()
        .enumerate()
        .map(|(position, name)| {
            (
                name.into(),
                Field {
                    position,
                    name: name.into(),
                    declaration: "usize".into(),
                    ty: TypeTree::Scalar("usize".into()),
                },
            )
        })
        .collect();
    let mut variants = vec![MixedVariant {
        position: 0,
        name: "OutputTooSmall".into(),
        shape: VariantShape::Named { fields },
    }];
    for (index, name) in [
        "LiteralOutOfBounds",
        "ExpectedAnotherByte",
        "OffsetZero",
        "OffsetOutOfBounds",
    ]
    .into_iter()
    .enumerate()
    {
        variants.push(MixedVariant {
            position: index + 1,
            name: name.into(),
            shape: VariantShape::Unit {},
        });
    }
    TypeTree::MixedEnum {
        nominal: Nominal {
            source_identity: Some("source".into()),
            package_name: "sample".into(),
            package_version: "1".into(),
            manifest_digest: concrete_types::digest(b"manifest"),
            definition_path: "Error".into(),
            declaration_digest: concrete_types::digest(SOURCE.as_bytes()),
        },
        source: SOURCE.into(),
        variants,
    }
}

fn original(source: &str) -> syn::ItemEnum {
    syn::parse_str(source).unwrap()
}

#[test]
fn complete_named_scalar_error_shape_preserves_all_variants() {
    let observed = tree();
    assert!(supported(&observed));
    declaration(&original(SOURCE), &observed).unwrap();
    for changed in [
        SOURCE.replace("expected: usize", "expected: Alias"),
        SOURCE.replace("expected: usize", "expected: DropBearing"),
        SOURCE.replace("actual: usize", "actual: &'static usize"),
        SOURCE.replace(", OffsetOutOfBounds", ""),
        SOURCE.replace(
            "OffsetZero, OffsetOutOfBounds",
            "OffsetOutOfBounds, OffsetZero",
        ),
        SOURCE.replace("OffsetZero", "OffsetZero = 4"),
        SOURCE.replace("enum Error", "enum Error<T>"),
        SOURCE.replace("actual: usize", "actual: usize, hidden: usize"),
    ] {
        assert!(
            declaration(&original(&changed), &observed).is_err(),
            "{changed}"
        );
    }
}

#[test]
fn observed_enum_cannot_omit_or_hide_named_payloads() {
    for mutation in ["variant", "field", "type", "name"] {
        let mut changed = tree();
        let TypeTree::MixedEnum { variants, .. } = &mut changed else {
            unreachable!()
        };
        match mutation {
            "variant" => {
                variants.pop();
            }
            _ => {
                let VariantShape::Named { fields } = &mut variants[0].shape else {
                    unreachable!()
                };
                match mutation {
                    "field" => {
                        fields.pop();
                    }
                    "type" => {
                        fields[0].1.ty =
                            TypeTree::Shared(Box::new(TypeTree::Scalar("usize".into())))
                    }
                    "name" => fields[0].0 = "forged".into(),
                    _ => unreachable!(),
                }
            }
        }
        assert!(
            declaration(&original(SOURCE), &changed).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn enum_shape_never_substitutes_for_complete_cleanup_closure() {
    for extra in [
        "impl Drop for Error { fn drop(&mut self) {} }",
        "fn hidden() { impl Drop for Error { fn drop(&mut self) {} } }",
        "type Alias = Error; impl Drop for Alias { fn drop(&mut self) {} }",
        "use core::ops::Drop as Cleanup; impl Cleanup for Error { fn drop(&mut self) {} }",
        "macro_rules! hidden { () => { impl Drop for Error { fn drop(&mut self) {} } }; } hidden!();",
    ] {
        let source = syn::parse_file(&format!("{SOURCE} {extra}")).unwrap();
        let items: Vec<_> = source.items.iter().collect();
        assert!(
            super::super::closure::inspect(&items, &[], &vec!["Error".into()]).is_err(),
            "{extra}"
        );
    }
}

fn different_shape(fields: &[(&str, &str)], unit_names: &[&str]) -> TypeTree {
    let mut observed = tree();
    let TypeTree::MixedEnum { variants, .. } = &mut observed else {
        unreachable!()
    };
    let fields = fields
        .iter()
        .enumerate()
        .map(|(position, (name, token))| {
            (
                (*name).into(),
                Field {
                    position,
                    name: (*name).into(),
                    declaration: (*token).into(),
                    ty: TypeTree::Scalar((*token).into()),
                },
            )
        })
        .collect();
    *variants = vec![MixedVariant {
        position: 0,
        name: "Detail".into(),
        shape: VariantShape::Named { fields },
    }];
    variants.extend(
        unit_names
            .iter()
            .enumerate()
            .map(|(index, name)| MixedVariant {
                position: index + 1,
                name: (*name).into(),
                shape: VariantShape::Unit {},
            }),
    );
    observed
}

#[test]
fn scalar_error_family_does_not_depend_on_fixture_counts_or_usize() {
    for (source, observed) in [
        (
            "enum Error { Detail { flag: bool }, Missing }",
            different_shape(&[("flag", "bool")], &["Missing"]),
        ),
        (
            "enum Error { Detail { left: i16, right: u64, valid: bool }, Missing, Invalid }",
            different_shape(
                &[("left", "i16"), ("right", "u64"), ("valid", "bool")],
                &["Missing", "Invalid"],
            ),
        ),
    ] {
        assert!(supported(&observed));
        declaration(&original(source), &observed).unwrap();
    }
}

#[test]
fn scalar_error_field_positions_and_spelling_remain_independent_evidence() {
    for attack in ["position", "spelling", "unit", "custom"] {
        let mut changed = different_shape(&[("flag", "bool")], &["Missing"]);
        let TypeTree::MixedEnum { variants, .. } = &mut changed else {
            unreachable!()
        };
        let VariantShape::Named { fields } = &mut variants[0].shape else {
            unreachable!()
        };
        match attack {
            "position" => fields[0].1.position = 1,
            "spelling" => fields[0].1.declaration = "Alias".into(),
            "unit" => fields[0].1.ty = TypeTree::Scalar("unit".into()),
            "custom" => fields[0].1.ty = tree(),
            _ => unreachable!(),
        }
        assert!(
            declaration(
                &original("enum Error { Detail { flag: bool }, Missing }"),
                &changed
            )
            .is_err()
        );
    }
}
