//! Source-pinned prelude identity, never inferred from the trait spelling alone.
use super::*;
use std::collections::BTreeMap;

const SOURCES: &[(&str, &str)] = &[
    (
        "core/src/prelude/mod.rs",
        "sha256:3db82d282c4ac980659183842d446e13d28b0bab609f3dc37e2036d9fefceb4c",
    ),
    (
        "core/src/prelude/v1.rs",
        "sha256:b57d120c6762622bd33114f46267dab8ba6c88be38f1433adb62d4ba36cd3a68",
    ),
    (
        "core/src/convert/mod.rs",
        "sha256:ec1dfed5b8eef22496fc85eeb0b26a5600944fb8d2f6d6e9aaf81fb45af06de5",
    ),
    (
        "std/src/prelude/mod.rs",
        "sha256:04e3eabaab3ff44016155b2949e7b6751547f9c480be51fb8a448ac8c9b2e105",
    ),
    (
        "std/src/prelude/v1.rs",
        "sha256:393900118db4000a0bd90f7c8f50fcb55475939586f24285ecca70925d6a6a2c",
    ),
    (
        "std/src/lib.rs",
        "sha256:5b6f8c7f657686c59e87c670d6b0df328e1aab622b56172821e486cc932175d5",
    ),
];

pub(super) const REFERENCE_NAMES: &[&str] =
    &["PartialEq", "Iterator", "DoubleEndedIterator", "TryFrom"];

/// Identity-only evidence; no implementation effect or call permission follows.
pub(super) struct ReferenceTrait {
    _private: (),
}

pub(super) fn reference_product(
    path: &syn::Path,
    scopes: &[BTreeMap<String, bool>],
    library: Option<&crate::data_library::Registry>,
    edition: Option<&str>,
) -> Option<ReferenceTrait> {
    let segment = path.segments.first()?;
    let name = segment.ident.to_string();
    if path.leading_colon.is_some()
        || path.segments.len() != 1
        || !reference_arguments(&name, &segment.arguments)
        || name == "TryFrom" && !matches!(edition, Some("2021" | "2024"))
        || scopes
            .iter()
            .any(|scope| scope.contains_key(&name) || scope.contains_key("*"))
        || !pinned(library?)
    {
        return None;
    }
    // The pinned core prelude reexports cmp::PartialEq and both iter traits;
    // pinned std prelude/root preserve those same core identities. The pinned
    // edition modules add convert::TryFrom specifically in 2021 and 2024.
    Some(ReferenceTrait { _private: () })
}

fn reference_arguments(name: &str, arguments: &syn::PathArguments) -> bool {
    match name {
        "PartialEq" | "TryFrom" => matches!(arguments, syn::PathArguments::AngleBracketed(args)
            if args.args.len() == 1 && matches!(args.args.first(), Some(syn::GenericArgument::Type(_)))),
        "Iterator" | "DoubleEndedIterator" => matches!(arguments, syn::PathArguments::None),
        _ => false,
    }
}

pub(super) fn resolve(
    path: &syn::Path,
    scopes: &[BTreeMap<String, bool>],
    library: Option<&crate::data_library::Registry>,
) -> Option<Namespace> {
    let segments = path.segments.iter().collect::<Vec<_>>();
    let [segment] = segments.as_slice() else {
        return None;
    };
    if path.leading_colon.is_some()
        || segment.ident != "From"
        || !matches!(&segment.arguments, syn::PathArguments::AngleBracketed(args)
            if args.args.len() == 1 && matches!(args.args.first(), Some(syn::GenericArgument::Type(_))))
        || scopes
            .iter()
            .any(|scope| scope.contains_key("From") || scope.contains_key("*"))
        || !pinned(library?)
    {
        return None;
    }
    Some(vec!["core".into(), "convert".into(), "From".into()])
}

pub(super) fn pinned(library: &crate::data_library::Registry) -> bool {
    SOURCES.iter().all(|(path, digest)| {
        library
            .cleanup_macro_source_digests
            .get(*path)
            .is_some_and(|actual| actual == digest)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prelude_identity_requires_every_reviewed_reexport_source() {
        assert!(pinned(&crate::data_library::registries()[0]));
        for (path, _) in SOURCES {
            let mut registry: crate::data_library::Registry = serde_json::from_str(include_str!(
                "../../../../../contracts/read-only-lengths-v2.json"
            ))
            .unwrap();
            registry.cleanup_macro_source_digests.remove(*path);
            assert!(!pinned(&registry), "{path}");
            registry
                .cleanup_macro_source_digests
                .insert((*path).into(), "sha256:changed".into());
            assert!(!pinned(&registry), "{path}");
        }
    }
}

#[cfg(test)]
mod reference_tests;
