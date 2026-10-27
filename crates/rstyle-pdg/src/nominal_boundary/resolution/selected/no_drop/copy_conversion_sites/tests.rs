//! Source-only expectations grant no Copy, core selection or admission authority.
use super::*;
use crate::concrete_types::{Field, MixedVariant, Nominal, VariantShape};

fn nominal(name: &str) -> Nominal {
    Nominal {
        source_identity: None,
        package_name: "source-only".into(),
        package_version: "0".into(),
        manifest_digest: "unverified".into(),
        definition_path: name.into(),
        declaration_digest: "unverified".into(),
    }
}

fn field(position: usize, name: &str, ty: TypeTree) -> Field {
    Field {
        position,
        name: name.into(),
        declaration: "source-only".into(),
        ty,
    }
}

pub(super) fn endpoints(direction: Direction) -> Endpoints {
    let product = TypeTree::Struct {
        nominal: nominal("ComponentRange"),
        source: "source-only".into(),
        fields: vec![
            field(0, "name", TypeTree::Shared(Box::new(TypeTree::Str))),
            field(1, "is_conditional", TypeTree::Scalar("bool".into())),
        ],
    };
    let tagged = TypeTree::MixedEnum {
        nominal: nominal("TryFromParsed"),
        source: "source-only".into(),
        variants: vec![
            MixedVariant {
                position: 0,
                name: "InsufficientInformation".into(),
                shape: VariantShape::Unit {},
            },
            MixedVariant {
                position: 1,
                name: "ComponentRange".into(),
                shape: VariantShape::SingleTuple {
                    field: field(0, "0", product.clone()),
                },
            },
        ],
    };
    match direction {
        Direction::Wrap => Endpoints {
            direction,
            input: product,
            output: tagged,
        },
        Direction::Unwrap => Endpoints {
            direction,
            input: tagged,
            output: TypeTree::Result {
                ok: Box::new(product),
                err: Box::new(TypeTree::Struct {
                    nominal: nominal("DifferentVariant"),
                    source: "struct DifferentVariant;".into(),
                    fields: vec![],
                }),
            },
        },
    }
}

fn source(source: &str, direction: Direction) -> Result<(), String> {
    let function = syn::parse_str::<syn::ItemFn>(source).unwrap();
    super::source::function(&function, &endpoints(direction))
}

#[test]
fn original_conversions_preserve_arbitrary_formal_and_payload_names() {
    for name in ["v", "supplied"] {
        assert!(source(&format!("fn from({name}: error::ComponentRange) -> Self {{ Self::ComponentRange({name}) }}"), Direction::Wrap).is_ok());
    }
    for name in ["err", "payload"] {
        assert!(source(&format!("fn try_from(input: TryFromParsed) -> Result<Self, Self::Error> {{ match input {{ TryFromParsed::ComponentRange({name}) => Ok({name}), _ => Err(error::DifferentVariant) }} }}"), Direction::Unwrap).is_ok());
    }
}

#[test]
fn rejects_wrapping_hidden_work_or_altered_parameter() {
    for function in [
        "fn from(v: T) -> Self { touch(); Self::ComponentRange(v) }",
        "fn from(v: T) -> Self { Self::ComponentRange(helper(v)) }",
        "fn from(v: T) -> Self { Self::ComponentRange(other) }",
        "fn from(v: T) -> Self { Self::Other(v) }",
        "fn from(v: T) -> Self { wrap!(v) }",
        "fn from(v: T) -> Self { return Self::ComponentRange(v); }",
        "fn from(mut v: T) -> Self { Self::ComponentRange(v) }",
        "fn from(ref v: T) -> Self { Self::ComponentRange(v) }",
        "fn from<T>(v: T) -> Self { Self::ComponentRange(v) }",
        "unsafe fn from(v: T) -> Self { Self::ComponentRange(v) }",
        "const fn from(v: T) -> Self { Self::ComponentRange(v) }",
    ] {
        assert!(source(function, Direction::Wrap).is_err(), "{function}");
    }
}

#[test]
fn inverse_requires_exact_payload_and_complete_unguarded_complement() {
    for body in [
        "match input { TryFromParsed::ComponentRange(v) => Ok(other), _ => Err(error::DifferentVariant) }",
        "match input { TryFromParsed::ComponentRange(v) if true => Ok(v), _ => Err(error::DifferentVariant) }",
        "match input { TryFromParsed::ComponentRange(ref v) => Ok(v), _ => Err(error::DifferentVariant) }",
        "match input { TryFromParsed::Other(v) => Ok(v), _ => Err(error::DifferentVariant) }",
        "match input { TryFromParsed::ComponentRange(v) => Ok(v) }",
        "match input { _ => Err(error::DifferentVariant), TryFromParsed::ComponentRange(v) => Ok(v) }",
        "match input { TryFromParsed::ComponentRange(v) => Ok(v), _ => Err(()) }",
        "match input { TryFromParsed::ComponentRange(v) => Ok(v), _ => Err(error::DifferentVariant()) }",
        "match input { TryFromParsed::ComponentRange(v) => { Ok(v) }, _ => Err(error::DifferentVariant) }",
        "match other { TryFromParsed::ComponentRange(v) => Ok(v), _ => Err(error::DifferentVariant) }",
    ] {
        let function = format!("fn try_from(input: T) -> Result<Self, Self::Error> {{ {body} }}");
        assert!(source(&function, Direction::Unwrap).is_err(), "{body}");
    }
}

#[test]
fn vocabulary_rejects_omitted_reordered_and_mismatched_payloads() {
    let mut endpoint = endpoints(Direction::Wrap);
    assert!(targets::vocabulary(&endpoint).is_ok());
    let TypeTree::MixedEnum { variants, .. } = &mut endpoint.output else {
        unreachable!()
    };
    variants.swap(0, 1);
    assert!(targets::vocabulary(&endpoint).is_err());
    let mut endpoint = endpoints(Direction::Wrap);
    let TypeTree::MixedEnum { variants, .. } = &mut endpoint.output else {
        unreachable!()
    };
    variants.pop();
    assert!(targets::vocabulary(&endpoint).is_err());
    let mut endpoint = endpoints(Direction::Wrap);
    endpoint.input = TypeTree::Scalar("unit".into());
    assert!(targets::vocabulary(&endpoint).is_err());
}

#[test]
fn nominal_unit_error_cannot_be_replaced_with_an_empty_aggregate() {
    let original = syn::parse_str::<syn::ItemStruct>("struct DifferentVariant;").unwrap();
    assert!(targets::nominal_unit(&original).is_ok());
    for source in [
        "struct DifferentVariant {}",
        "struct DifferentVariant();",
        "struct DifferentVariant(u8);",
    ] {
        let item = syn::parse_str::<syn::ItemStruct>(source).unwrap();
        assert!(targets::nominal_unit(&item).is_err(), "{source}");
    }
}
