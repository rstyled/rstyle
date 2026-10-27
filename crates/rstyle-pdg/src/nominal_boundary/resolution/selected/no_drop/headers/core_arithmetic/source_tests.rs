use super::*;
use proc_macro2::{TokenStream, TokenTree};

#[test]
fn reviewed_complete_core_arithmetic_sources_distinguish_traits_from_drop() {
    let output = std::process::Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let sysroot = String::from_utf8(output.stdout).unwrap();
    let base =
        std::path::Path::new(sysroot.trim()).join("lib/rustlib/src/rust/library/core/src/ops");
    let mut sources = Vec::new();
    for (name, checksum) in [
        ("mod.rs", "xxh3-64:8821a70a1b916d76"),
        ("arith.rs", "xxh3-64:318d2d75d11d593d"),
    ] {
        let bytes = std::fs::read(base.join(name)).unwrap();
        assert_eq!(crate::source_identity::checksum(&bytes), checksum, "{name}");
        sources.push(String::from_utf8(bytes).unwrap());
    }
    verify_reexports(&sources[0]);
    verify_declarations(&sources[1]);
}

fn verify_reexports(source: &str) {
    let tokens = source
        .parse::<TokenStream>()
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>();
    let mut arithmetic = Vec::new();
    for window in tokens.windows(10) {
        let Ok(item) = syn::parse2::<syn::ItemUse>(window.iter().cloned().collect()) else {
            continue;
        };
        if !matches!(item.vis, syn::Visibility::Public(_)) {
            continue;
        }
        arithmetic.extend(
            source_imports::bindings(&item.tree)
                .into_iter()
                .filter(|(_, path)| path.starts_with(&["self".into(), "arith".into()]))
                .map(|(name, _)| name),
        );
    }
    for name in NAMES {
        assert!(arithmetic.iter().any(|actual| actual == name), "{name}");
    }
    assert!(!arithmetic.iter().any(|name| name == "Drop"));
    assert!(source.contains("pub use self::drop::Drop;"));
}

fn verify_declarations(source: &str) {
    let tokens = source
        .parse::<TokenStream>()
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>();
    for name in NAMES {
        assert_eq!(tokens.windows(4).filter(|window| matches!(window,
            [TokenTree::Ident(public), TokenTree::Ident(constant), TokenTree::Ident(keyword), TokenTree::Ident(actual)]
                if public == "pub" && constant == "const" && keyword == "trait" && actual == name))
            .count(), 1, "{name}");
    }
    assert!(!tokens.windows(2).any(|window| matches!(window,
        [TokenTree::Ident(keyword), TokenTree::Ident(name)] if keyword == "trait" && name == "Drop")));
}
