use super::*;

fn library_source(imports: &str, body: &str) -> String {
    format!("extern crate alloc; struct Error(()); {imports} {WITNESS} fn neighbor() {{ {body} }}")
}

#[test]
fn pinned_alloc_macros_reach_the_complete_argument_walker() {
    for (imports, body) in [
        ("use alloc::vec;", "let _ = vec![1, 2];"),
        ("use alloc::{vec, vec::Vec};", "let _ = vec![0; 2];"),
        ("use alloc::format;", "let _ = format!(\"{}\", 1);"),
        ("", "let _ = alloc::vec![];"),
        ("", "let _ = alloc::format!(\"{}\", 1);"),
    ] {
        verify(&library_source(imports, body)).unwrap();
    }
}

#[test]
fn pinned_alloc_macro_arguments_cannot_hide_cleanup() {
    for body in [
        "let _ = vec![{ impl Drop for Error { fn drop(&mut self) {} } 1 }];",
        "let _ = vec![0; { impl Drop for Error { fn drop(&mut self) {} } 1 }];",
        "let _ = format!(\"{}\", { impl Drop for Error { fn drop(&mut self) {} } 1 });",
    ] {
        assert!(verify(&library_source("use alloc::{vec, format};", body)).is_err());
    }
}

#[test]
fn library_macro_route_does_not_bypass_lexical_ownership() {
    for (imports, body) in [
        ("use external::vec;", "let _ = vec![];"),
        ("use alloc::vec as format;", "let _ = format![];"),
        ("use alloc::vec; use external::*;", "let _ = vec![];"),
        (
            "use alloc::vec;",
            "use external::hidden as vec; let _ = vec![];",
        ),
        ("use alloc::vec;", "use external::*; let _ = vec![];"),
        ("mod alloc {}", "let _ = alloc::vec![];"),
        ("type alloc = ();", "let _ = alloc::vec![];"),
        (
            "",
            "macro_rules! vec { () => {{ impl Drop for Error { fn drop(&mut self) {} } }} } vec!();",
        ),
    ] {
        assert!(
            verify(&library_source(imports, body)).is_err(),
            "{imports} {body}"
        );
    }
}

#[test]
fn library_macro_route_requires_exact_export_evidence() {
    let text = library_source("use alloc::vec;", "let _ = vec![];");
    let (environment, mut facts) = fixture(&text);
    facts.build.rustc_commit = crate::frontend::unknown_str("unregistered");
    assert!(
        super::super::verify_reference_product(&environment, &facts, &vec!["Error".into()])
            .is_err()
    );
}

#[test]
fn library_binding_rejects_missing_changed_or_private_exports() {
    let text = library_source("use alloc::vec;", "let _ = vec![];");
    let (environment, facts) = fixture(&text);
    let resolver = source_macros::SourceMacros::new(&environment, &facts.build.cfgs, None).unwrap();
    let invocation: syn::ExprMacro = syn::parse_str("vec![]").unwrap();
    let registry = crate::data_library::registry_for(&facts).unwrap();
    assert!(
        resolver
            .library_invocation(&[], &invocation.mac, Some(registry))
            .unwrap()
    );

    for change in 0..4 {
        let mut changed: crate::data_library::Registry =
            serde_json::from_str(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/contracts/read-only-lengths-v2.json"
            )))
            .unwrap();
        let exports = changed.alloc_macro_exports.as_mut().unwrap();
        let item = exports
            .macros
            .iter_mut()
            .find(|item| item.name == "vec")
            .unwrap();
        match change {
            0 => changed.alloc_macro_exports = None,
            1 => item.public = false,
            2 => item.definition.crate_hash = "changed".into(),
            _ => item.definition.path = "alloc::other".into(),
        }
        assert!(
            !resolver
                .library_invocation(&[], &invocation.mac, Some(&changed))
                .unwrap()
        );
    }
}

#[test]
fn imported_native_tls_keeps_platform_and_initializer_checks() {
    let text = library_source(
        "use std::thread_local;",
        "thread_local!(static VALUE: usize = 1;);",
    );
    let (environment, mut facts) = fixture(&text);
    let target = vec!["Error".into()];

    assert!(super::super::verify_reference_product(&environment, &facts, &target).is_err());
    facts.build.cfgs.push("target_thread_local".into());
    super::super::verify_reference_product(&environment, &facts, &target).unwrap();
    let changed = text.replace(
        "= 1;",
        "= { impl Drop for Error { fn drop(&mut self) {} } 1 };",
    );
    let (environment, mut facts) = fixture(&changed);
    facts.build.cfgs.push("target_thread_local".into());
    assert!(super::super::verify_reference_product(&environment, &facts, &target).is_err());
}
