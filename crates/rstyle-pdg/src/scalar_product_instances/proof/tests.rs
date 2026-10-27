//! Independent source/cleanup adversaries; these tests do not mint owner receipts.
use super::*;
use crate::concrete_types::{Field, Nominal};

const ORIGINAL: &str = include_str!("original-parsed-item.rs");

fn inspect(source: &str) -> Result<(), String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let items = file.items.iter().collect::<Vec<_>>();
    closure::inspect_reference_product(
        &items,
        &vec!["ParsedItem".into()],
        Some(crate::data_library::registry()),
    )
}

#[test]
fn original_non_copy_tuple_and_all_generic_neighbor_impls_have_closed_cleanup_source() {
    assert!(ORIGINAL.contains("struct ParsedItem<'a, T>"));
    assert!(ORIGINAL.contains("impl<'a, T> ParsedItem<'a, T>"));
    assert!(ORIGINAL.contains("impl<'a> ParsedItem<'a, Option<()>>"));
    assert!(!ORIGINAL.contains("derive(Copy"));
    assert!(inspect(ORIGINAL).is_ok());
}

#[test]
fn generic_drop_direct_qualified_aliased_and_generated_forms_remain_rejected() {
    for extra in [
        "impl<'a,T> Drop for ParsedItem<'a,T> { fn drop(&mut self) {} }",
        "impl<'a,T> core::ops::Drop for ParsedItem<'a,T> { fn drop(&mut self) {} }",
        "use core::ops::Drop as Cleanup; impl<'a,T> Cleanup for ParsedItem<'a,T> { fn drop(&mut self) {} }",
        "type Alias<'a,T> = ParsedItem<'a,T>; impl<'a,T> Drop for Alias<'a,T> { fn drop(&mut self) {} }",
        "impl<T> core::ops::Drop for T { fn drop(&mut self) {} }",
        "macro_rules! add_cleanup { () => { impl<'a,T> Drop for ParsedItem<'a,T> { fn drop(&mut self) {} } }; } add_cleanup!();",
    ] {
        assert!(inspect(&format!("{ORIGINAL}\n{extra}")).is_err(), "{extra}");
    }
}

#[test]
fn hidden_neighbor_items_and_unknown_expansions_cannot_prove_drop_absence() {
    for extra in [
        "fn neighbor() { impl<'a,T> Drop for ParsedItem<'a,T> { fn drop(&mut self) {} } }",
        "unknown_source!();",
        "#[unknown_attribute] struct Neighbor;",
        "#[derive(CustomDerive)] struct Neighbor;",
        "macro_rules! Debug { () => {} }",
    ] {
        assert!(inspect(&format!("{ORIGINAL}\n{extra}")).is_err(), "{extra}");
    }
}

#[test]
fn exact_const_impl_identity_is_independent_of_observed_result_argument() {
    for (digits, argument) in [(1, Argument::U8), (2, Argument::U8), (3, Argument::U16)] {
        let item: syn::ItemImpl =
            syn::parse_str(&format!("impl ExactlyNDigits<{digits}> {{}}")).unwrap();
        assert_eq!(source::concrete_owner(&item, argument).unwrap(), digits);
    }
    for source in [
        "impl ExactlyNDigits<0> {}",
        "impl ExactlyNDigits<4> {}",
        "impl ExactlyNDigits<3> {}",
        "impl ExactlyNDigits<N> {}",
        "impl ExactlyNDigits<{1 + 0}> {}",
        "impl ExactlyNDigits<1u8> {}",
        "impl Other<1> {}",
        "impl other::ExactlyNDigits<1> {}",
        "impl<T> ExactlyNDigits<1> {}",
        "impl Trait for ExactlyNDigits<1> {}",
    ] {
        let item: syn::ItemImpl = syn::parse_str(source).unwrap();
        assert!(
            source::concrete_owner(&item, Argument::U8).is_err(),
            "{source}"
        );
    }
}

#[test]
fn lexical_byte_shadowing_and_unresolved_globs_are_rejected() {
    for source in [
        "type u8 = u16;",
        "struct u8;",
        "use other::Byte as u8;",
        "type u16 = u32;",
        "struct u16;",
        "enum u16 {}",
        "union u16 { value: u32 }",
        "trait u16 {}",
        "mod u16 {}",
        "extern crate other as u16;",
        "use other::Short as u16;",
        "use other::*;",
    ] {
        let item: syn::Item = syn::parse_str(source).unwrap();
        assert!(source::byte_binding(&item).is_err(), "{source}");
    }
    let item: syn::Item = syn::parse_str("use crate::parsing::ParsedItem;").unwrap();
    assert!(source::byte_binding(&item).is_ok());
}

fn instance(argument: Argument) -> Instance {
    let source: crate::SourceText = "#[derive(Debug)] struct ParsedItem<'a,T>(&'a [u8],T);".into();
    Instance {
        nominal: Nominal {
            source_identity: Some("original-package".into()),
            package_name: "time".into(),
            package_version: "0.3.47".into(),
            manifest_digest: crate::source_identity::checksum(b"manifest"),
            definition_path: "parsing::ParsedItem".into(),
            declaration_digest: source.checksum().into(),
        },
        source,
        argument,
        fields: vec![
            Field {
                position: 0,
                name: "0".into(),
                declaration: "&'a [u8]".into(),
                ty: TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(TypeTree::Scalar(
                    "u8".into(),
                ))))),
            },
            Field {
                position: 1,
                name: "1".into(),
                declaration: "T".into(),
                ty: TypeTree::Scalar(argument.as_str().into()),
            },
        ],
    }
}

#[test]
fn mixed_instance_inventory_and_missing_result_evidence_do_not_gain_cleanup_permission() {
    let byte = instance(Argument::U8);
    let short = instance(Argument::U16);
    let payload = TypeTree::ScalarProductInstance(Box::new(byte.clone()));
    let mut interface = InterfaceObservations {
        identity: concrete_types::IDENTITY.into(),
        parameters: vec![],
        result: Observation::Captured {
            ty: TypeTree::Option(Box::new(payload)),
        },
    };
    assert!(verify_interface(&interface, &byte).is_ok());
    assert!(verify_interface(&interface, &short).is_err());
    interface.parameters.push(Observation::Captured {
        ty: TypeTree::Tuple(vec![TypeTree::ScalarProductInstance(Box::new(short))]),
    });
    assert!(verify_interface(&interface, &byte).is_err());
    interface.result = Observation::Unsupported {
        reason: "missing actual result".into(),
    };
    assert!(verify_interface(&interface, &byte).is_err());
}
