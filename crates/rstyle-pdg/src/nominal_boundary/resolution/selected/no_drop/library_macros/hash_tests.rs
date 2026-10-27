use super::*;
use proc_macro2::{Delimiter, TokenStream, TokenTree};

fn registry() -> Registry {
    serde_json::from_str(include_str!(
        "../../../../../../contracts/read-only-lengths-v2.json"
    ))
    .unwrap()
}

#[test]
fn hash_reexport_requires_the_reviewed_revision_and_existing_prelude_authority() {
    let prefix = ["core".into(), "hash".into()];
    for registry in crate::data_library::registries() {
        assert!(core_hash(&prefix, "Hash", Some(registry)));
    }
    let mut registry = registry();
    registry.rustc_commit = "different-compiler".into();
    assert!(!core_hash(&prefix, "Hash", Some(&registry)));
    registry.rustc_commit = HASH_REEXPORT_COMMIT.into();
    registry
        .cleanup_macro_source_digests
        .remove("core/src/prelude/v1.rs");
    assert!(!core_hash(&prefix, "Hash", Some(&registry)));
    registry
        .cleanup_macro_source_digests
        .insert("core/src/prelude/v1.rs".into(), "changed-source".into());
    assert!(!core_hash(&prefix, "Hash", Some(&registry)));
    assert!(!core_hash(&prefix, "Hash", None));
}

#[test]
fn reviewed_hash_source_retains_builtin_definition_and_public_reexport() {
    let output = std::process::Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let sysroot = String::from_utf8(output.stdout).unwrap();
    let path = std::path::Path::new(sysroot.trim())
        .join("lib/rustlib/src/rust/library/core/src/hash/mod.rs");
    let bytes = std::fs::read(path).unwrap();
    // Source-review regression only; this checksum never runs in extraction.
    assert_eq!(
        crate::source_identity::checksum(&bytes),
        "xxh3-64:1a60cb2173d0aed0"
    );
    let tokens = std::str::from_utf8(&bytes)
        .unwrap()
        .parse::<TokenStream>()
        .unwrap();
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    verify_builtin(&tokens);
    assert_eq!(
        tokens
            .windows(7)
            .filter(|window| public_reexport(window))
            .count(),
        1
    );
}

fn public_reexport(tokens: &[TokenTree]) -> bool {
    if !matches!(&tokens[..2], [TokenTree::Ident(visibility), TokenTree::Ident(keyword)]
        if visibility == "pub" && keyword == "use")
    {
        return false;
    }
    let Ok(item) = syn::parse2::<syn::ItemUse>(tokens.iter().cloned().collect()) else {
        return false;
    };
    let syn::UseTree::Path(path) = &item.tree else {
        return false;
    };
    matches!(item.vis, syn::Visibility::Public(_))
        && item.leading_colon.is_none()
        && path.ident == "macros"
        && matches!(path.tree.as_ref(), syn::UseTree::Name(name) if name.ident == "Hash")
}

fn verify_builtin(tokens: &[TokenTree]) {
    let modules = tokens
        .windows(3)
        .filter_map(|tokens| match tokens {
            [
                TokenTree::Ident(keyword),
                TokenTree::Ident(name),
                TokenTree::Group(body),
            ] if keyword == "mod" && name == "macros" && body.delimiter() == Delimiter::Brace => {
                Some(body.stream())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let [module] = modules.as_slice() else {
        panic!("one original macros module");
    };
    let module = module.clone().into_iter().collect::<Vec<_>>();
    assert!(module.windows(2).any(|tokens| matches!(tokens,
        [TokenTree::Punct(hash), TokenTree::Group(attribute)] if hash.as_char() == '#'
            && attribute.delimiter() == Delimiter::Bracket && attribute.stream().to_string() == "rustc_builtin_macro")));
    assert_eq!(
        module
            .windows(5)
            .filter(|tokens| builtin_declaration(tokens))
            .count(),
        1
    );
}

fn builtin_declaration(tokens: &[TokenTree]) -> bool {
    matches!(tokens,
        [TokenTree::Ident(visibility), TokenTree::Ident(keyword), TokenTree::Ident(name), TokenTree::Group(input), TokenTree::Group(body)]
            if visibility == "pub" && keyword == "macro" && name == "Hash"
                && input.delimiter() == Delimiter::Parenthesis && input.stream().to_string() == "$ item : item"
                && body.delimiter() == Delimiter::Brace && body.stream().is_empty())
}

#[test]
fn exact_hash_imports_keep_historical_shadow_and_cleanup_barriers() {
    let source = "struct Error(()); use core::hash::Hash; #[derive(Hash)] struct Value;";
    assert!(super::super::closure::inspect_external_source(source, true).is_ok());
    assert!(super::super::closure::inspect_external_source(source, false).is_err());
    for extra in [
        "mod core {} use core::hash::Hash;",
        "use external::hash::Hash;",
        "use core::hash::Hash as Hash;",
        "use core::hash::Hash as Ord;",
        "use core::hash::*; #[derive(Hash)] struct Value;",
        "use core::hash::Hash; impl Drop for Error { fn drop(&mut self) {} }",
        "fn nested() { use core::hash::Hash; impl Drop for Error { fn drop(&mut self) {} } }",
        "use core::hash::Hash; macro_rules! hidden { () => { impl Drop for Error { fn drop(&mut self) {} } }; } hidden!();",
    ] {
        let source = format!("struct Error(()); {extra}");
        assert!(
            super::super::closure::inspect_external_source(&source, true).is_err(),
            "{source}"
        );
    }
}
