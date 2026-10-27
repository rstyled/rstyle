//! Prelude trait identity excludes Drop without certifying implementation effects.
use super::super::closure;
use super::*;

fn inspect(source: &str, reference_product: bool) -> Result<(), String> {
    let file = syn::parse_file(source).unwrap();
    let items = file.items.iter().collect::<Vec<_>>();
    let target = vec!["Error".into()];
    if reference_product {
        closure::inspect_reference_product(
            &items,
            &target,
            crate::data_library::registries().first(),
        )
    } else {
        closure::inspect_closed_imports(&items, &target)
    }
}

fn implementation(name: &str) -> String {
    let iterator =
        "impl Iterator for Error { type Item = u8; fn next(&mut self) -> Option<u8> { None } }";
    match name {
        "PartialEq" => {
            "impl PartialEq<u8> for Error { fn eq(&self, _: &u8) -> bool { true } }".into()
        }
        "Iterator" => iterator.into(),
        "TryFrom" => "impl TryFrom<u8> for Error { type Error = (); fn try_from(_: u8) -> Result<Self, ()> { Ok(Error) } }".into(),
        "DoubleEndedIterator" => format!(
            "{iterator} impl DoubleEndedIterator for Error {{ fn next_back(&mut self) -> Option<u8> {{ None }} }}"
        ),
        _ => unreachable!(),
    }
}

#[test]
fn exact_pinned_reference_prelude_traits_preserve_historical_rejection() {
    for name in REFERENCE_NAMES {
        let source = format!("struct Error; {}", implementation(name));
        inspect(&source, true).unwrap();
        assert!(
            inspect(&source, false)
                .unwrap_err()
                .contains("possible destructor")
        );
    }
    let generated = "struct Error; macro_rules! iter { ($name:ident) => { impl Iterator for $name { type Item = u8; fn next(&mut self) -> Option<u8> { None } } }; } iter!(Error);";
    inspect(generated, true).unwrap();
    assert!(inspect(generated, false).is_err());
}

#[test]
fn try_from_requires_known_edition_specific_prelude_ownership() {
    let source = format!("struct Error; {}", implementation("TryFrom"));
    let file = syn::parse_file(&source).unwrap();
    let items = file.items.iter().collect::<Vec<_>>();
    let target = vec!["Error".into()];
    let library = crate::data_library::registries().first();
    for edition in [Some("2021"), Some("2024")] {
        closure::inspect_reference_product_edition(&items, &target, library, edition).unwrap();
    }
    for edition in [
        None,
        Some("2015"),
        Some("2018"),
        Some("2027"),
        Some("unknown"),
    ] {
        let error = closure::inspect_reference_product_edition(&items, &target, library, edition)
            .unwrap_err();
        assert!(
            error.contains("possible destructor"),
            "{edition:?}: {error}"
        );
    }
}

#[test]
fn generic_header_syntax_is_checked_without_erasing_arguments() {
    let registry = &crate::data_library::registries()[0];
    for spelling in [
        "PartialEq<u8>",
        "PartialEq<core::ops::Range<usize>>",
        "Iterator",
        "DoubleEndedIterator",
        "TryFrom<u8>",
    ] {
        let path = syn::parse_str(spelling).unwrap();
        assert!(
            reference_product(&path, &[], Some(registry), Some("2021")).is_some(),
            "{spelling}"
        );
    }
    for spelling in [
        "PartialEq",
        "PartialEq<>",
        "PartialEq<u8,u16>",
        "PartialEq<'a>",
        "PartialEq<Item=u8>",
        "Iterator<>",
        "Iterator<u8>",
        "Iterator<Item=u8>",
        "DoubleEndedIterator<u8>",
        "core::cmp::PartialEq<u8>",
        "::Iterator",
        "Drop",
        "From<u8>",
        "TryFrom",
        "TryFrom<>",
        "TryFrom<u8,u16>",
        "TryFrom<'a>",
        "TryFrom<Item=u8>",
    ] {
        let path = syn::parse_str(spelling).unwrap();
        assert!(
            reference_product(&path, &[], Some(registry), Some("2021")).is_none(),
            "{spelling}"
        );
    }
}

#[test]
fn every_existing_prelude_source_pin_is_required_for_each_identity() {
    for spelling in [
        "PartialEq<u8>",
        "Iterator",
        "DoubleEndedIterator",
        "TryFrom<u8>",
    ] {
        let path = syn::parse_str(spelling).unwrap();
        for (source, _) in SOURCES {
            let mut registry: crate::data_library::Registry = serde_json::from_str(include_str!(
                "../../../../../../contracts/read-only-lengths-v2.json"
            ))
            .unwrap();
            registry.cleanup_macro_source_digests.remove(*source);
            assert!(
                reference_product(&path, &[], Some(&registry), Some("2021")).is_none(),
                "{spelling}: {source}"
            );
            registry
                .cleanup_macro_source_digests
                .insert((*source).into(), "sha256:changed".into());
            assert!(
                reference_product(&path, &[], Some(&registry), Some("2021")).is_none(),
                "{spelling}: {source}"
            );
        }
    }
}

#[test]
fn aliases_generated_bindings_and_unknown_lexical_globs_cannot_supply_identity() {
    for name in REFERENCE_NAMES {
        let implementation = implementation(name);
        for binding in [
            format!("use core::ops::Drop as {name};"),
            format!("use external::{name};"),
            format!("type {name} = Unknown;"),
            format!("struct {name};"),
        ] {
            let ordinary = format!("struct Error; {binding} {implementation}");
            assert!(inspect(&ordinary, true).is_err(), "{ordinary}");
            let generated = format!(
                "struct Error; macro_rules! shadow {{ () => {{ {binding} }}; }} shadow!(); {implementation}"
            );
            assert!(inspect(&generated, true).is_err(), "{generated}");
        }
        for source in [
            format!("struct Error; fn hidden() {{ use external::*; {implementation} }}"),
            format!(
                "struct Error; macro_rules! shadow {{ () => {{ use external::*; }}; }} shadow!(); {implementation}"
            ),
            format!("struct Error; fn neighboring() {{ use external::{name}; }} {implementation}"),
        ] {
            assert!(inspect(&source, true).is_err(), "{source}");
        }
        let siblings =
            format!("struct Error; fn neighboring() {{ use external::*; }} {implementation}");
        inspect(&siblings, true).unwrap();
    }
}

#[test]
fn trait_identity_never_omits_hidden_destructors_or_impl_bodies() {
    for name in REFERENCE_NAMES {
        let implementation = implementation(name);
        let source = format!(
            "struct Error; {implementation} fn hidden() {{ impl Drop for Error {{ fn drop(&mut self) {{}} }} }}"
        );
        assert!(inspect(&source, true).is_err(), "{source}");
        let generated = format!(
            "struct Error; {implementation} macro_rules! hidden {{ () => {{ impl Drop for Error {{ fn drop(&mut self) {{}} }} }}; }} hidden!();"
        );
        assert!(inspect(&generated, true).is_err(), "{generated}");
    }
    let in_body = "struct Error; impl Iterator for Error { type Item = u8; fn next(&mut self) -> Option<u8> { impl Drop for Error { fn drop(&mut self) {} } None } }";
    assert!(inspect(in_body, true).is_err());
}
