//! Pinned source-only expansion contracts; never function-effect authority.
use crate::data_library::Registry;
use std::collections::BTreeMap;
#[cfg(test)]
mod core_cmp_tests;
#[cfg(test)]
mod hash_tests;

const HASH_REEXPORT_COMMIT: &str = "5c543b0b8c73c7b72bc8284ced4fb22ead15734d";

/// Reviewed public Hash reexport preserves the prelude's builtin derive.
/// The revision is explicit because the reexport source is checked in tests;
/// runtime selection performs neither source IO nor hashing.
pub(super) fn core_hash(prefix: &[String], name: &str, registry: Option<&Registry>) -> bool {
    prefix == ["core", "hash"]
        && name == "Hash"
        && registry.is_some_and(|registry| {
            registry.rustc_commit == HASH_REEXPORT_COMMIT && super::prelude_traits::pinned(registry)
        })
}

/// The checked prelude reexports these exact trait and derive bindings.
/// Callers separately gate the source profile and prove the library root.
pub(super) fn core_cmp(prefix: &[String], name: &str, registry: Option<&Registry>) -> bool {
    prefix == ["core", "cmp"]
        && matches!(name, "Eq" | "Ord" | "PartialEq" | "PartialOrd")
        && registry.is_some_and(super::prelude_traits::pinned)
}

const SOURCES: &[(&str, &str)] = &[
    (
        "std/src/thread/local.rs",
        "sha256:e3ab1f6bc155548843ec942bc409aecf2d812e6a4088680a4647df1848b5b1f1",
    ),
    (
        "std/src/sys/thread_local/mod.rs",
        "sha256:b8e407313093a89008fd0fbdd4805f2da8c7bb7711f0d36c03d858d6f0de5e4a",
    ),
    (
        "std/src/sys/thread_local/native/mod.rs",
        "sha256:258d21c8714e9e51ab4bba15b086dd5a83acad4573ac597887e5196c30adbeec",
    ),
];

pub(super) fn pinned_native(registry: &Registry) -> bool {
    [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "arm64-apple-macosx",
    ]
    .contains(&registry.target.as_str())
        && SOURCES.iter().all(|(path, digest)| {
            registry
                .cleanup_macro_source_digests
                .get(*path)
                .is_some_and(|actual| actual == digest)
        })
}

pub(super) fn import(prefix: &[String], name: &str, registry: Option<&Registry>) -> bool {
    let Some(registry) = registry else {
        return false;
    };
    match (
        prefix
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice(),
        name,
    ) {
        (["alloc"], "vec" | "format") => {
            registry
                .alloc_macro_exports
                .as_ref()
                .is_some_and(|exports| {
                    exports.library.crate_name == "alloc"
                        && exports.macros.iter().any(|item| {
                            item.name == name
                                && item.kind == "bang"
                                && item.public
                                && item.definition.crate_name == "alloc"
                                && item.definition.crate_hash == exports.library.crate_hash
                                && item.definition.path == format!("alloc::{name}")
                        })
                })
        }
        // These exact bytes define `#[rustc_builtin_macro] pub macro Debug`
        // and reexport it beside the trait; this is derive authority, not a
        // claim that the imported name is absent from the macro namespace.
        (["core", "fmt"], "Debug") => registry
            .cleanup_macro_source_digests
            .get("core/src/fmt/mod.rs")
            .is_some_and(|digest| {
                digest == "sha256:ab918e40543739be20757973f0d4abefcd9624d3759a64fc4147f1d8fc4bc035"
            }),
        (["std"], "thread_local") => pinned_native(registry),
        _ => false,
    }
}

/// Exact reviewed std -> alloc -> core reexports preserve both Debug namespaces.
pub(super) fn std_debug(registry: Option<&Registry>) -> bool {
    let Some(registry) = registry else {
        return false;
    };
    import(&["core".into(), "fmt".into()], "Debug", Some(registry))
        && [
            (
                "std/src/lib.rs",
                "sha256:5b6f8c7f657686c59e87c670d6b0df328e1aab622b56172821e486cc932175d5",
            ),
            (
                "alloc/src/lib.rs",
                "sha256:3b17b423a626ffa5ff114a1f1ebab97df8998b3189753f1a187846c35c0d25c8",
            ),
            (
                "alloc/src/fmt.rs",
                "sha256:cd07ff6ed15cf6b8b31b7d601b638cf4c6ca9f0d3e8963b33d170160f4b0cf06",
            ),
        ]
        .iter()
        .all(|(path, digest)| {
            registry
                .cleanup_macro_source_digests
                .get(*path)
                .is_some_and(|actual| actual == digest)
        })
}

/// Only these exact pinned alloc paths reuse the existing reviewed argument
/// grammar. Arbitrary qualified macro paths retain their unsupported outcome.
pub(super) fn qualified_alloc(mac: &syn::Macro, registry: Option<&Registry>) -> Option<syn::Macro> {
    let parts: Vec<_> = mac.path.segments.iter().collect();

    let [root, name] = parts.as_slice() else {
        return None;
    };
    if root.ident != "alloc"
        || !matches!(root.arguments, syn::PathArguments::None)
        || !matches!(name.arguments, syn::PathArguments::None)
    {
        return None;
    }
    let name = name.ident.to_string();
    if !["vec", "format"].contains(&name.as_str()) || !import(&["alloc".into()], &name, registry) {
        return None;
    }
    let mut normalized = mac.clone();
    normalized.path = syn::Path::from(syn::Ident::new(&name, parts[1].ident.span()));
    Some(normalized)
}

pub(super) fn bindings(
    tree: &syn::UseTree,
    prefix: &[String],
    scope: &mut BTreeMap<String, bool>,
    registry: Option<&Registry>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            let child: Vec<_> = prefix
                .iter()
                .cloned()
                .chain([path.ident.to_string()])
                .collect();
            bindings(&path.tree, &child, scope, registry);
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                bindings(item, prefix, scope, registry);
            }
        }
        _ => binding_leaf(tree, prefix, scope, registry),
    }
}

fn binding_leaf(
    tree: &syn::UseTree,
    prefix: &[String],
    scope: &mut BTreeMap<String, bool>,
    registry: Option<&Registry>,
) {
    match tree {
        syn::UseTree::Name(name) if prefix == ["core", "ops"] && name.ident == "Range"
            && registry.is_some_and(|registry| registry.source_digests.get("ops/range.rs").is_some_and(|digest| digest == "sha256:5d791bd2ee8997e93ac72c35db831815d9acbb7aed05cd1539799b7677a13d20")) => {
            // Exact pinned standard Range declaration cannot be the selected
            // local unit error. This does not infer anything about From.
            scope.insert("Range".into(), true);
        }
        syn::UseTree::Name(name) if prefix == ["std"] && name.ident == "thread_local" => {
            scope.insert("macro:thread_local".into(), true);
        }
        _ => {}
    }
}

/// Reviewed native inner emits only LocalKey/initializer/storage items. For a
/// source-unshadowed scalar type, none can implement Drop for the selected owner.
/// Initializers and attributes remain original syntax and must be visited whole.
pub(super) fn scalar_statics(mac: &syn::Macro) -> Result<Vec<syn::Item>, String> {
    let file = syn::parse2::<syn::File>(mac.tokens.clone()).map_err(|error| error.to_string())?;
    for item in &file.items {
        let syn::Item::Static(item) = item else {
            return Err("Cleanup thread_local requires complete scalar static declarations".into());
        };
        if !matches!(item.mutability, syn::StaticMutability::None)
            || scalar_name(&item.ty).is_none()
        {
            return Err("Cleanup thread_local requires immutable primitive scalar statics".into());
        }
    }
    Ok(file.items)
}

pub(super) fn scalar_name(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    let name = path.path.get_ident()?.to_string();
    (path.qself.is_none() && crate::concrete_types::scalar(&name)).then_some(name)
}
