use super::*;

const NAMES: &[&str] = &["Eq", "Ord", "PartialEq", "PartialOrd"];

#[test]
fn exact_comparison_bindings_reuse_existing_prelude_authority() {
    for registry in crate::data_library::registries() {
        for name in NAMES {
            assert!(core_cmp(
                &["core".into(), "cmp".into()],
                name,
                Some(registry)
            ));
        }
    }
    for prefix in [
        vec!["std", "cmp"],
        vec!["external", "cmp"],
        vec!["core", "other"],
    ] {
        let prefix = prefix.into_iter().map(str::to_owned).collect::<Vec<_>>();
        assert!(!core_cmp(
            &prefix,
            "Ord",
            Some(crate::data_library::registry())
        ));
    }
    assert!(!core_cmp(&["core".into(), "cmp".into()], "Ord", None));
    assert!(!core_cmp(
        &["core".into(), "cmp".into()],
        "Ordering",
        Some(crate::data_library::registry())
    ));
}

fn registry() -> Registry {
    serde_json::from_str(include_str!(
        "../../../../../../contracts/read-only-lengths-v2.json"
    ))
    .unwrap()
}

#[test]
fn missing_or_changed_prelude_source_cannot_certify_comparison_imports() {
    let sources = registry().cleanup_macro_source_digests;
    for path in sources.keys().filter(|path| {
        path.contains("prelude")
            || path.ends_with("convert/mod.rs")
            || path.ends_with("std/src/lib.rs")
    }) {
        for replacement in [None, Some("changed-source".to_owned())] {
            let mut registry = registry();
            registry.cleanup_macro_source_digests.remove(path);
            if let Some(value) = replacement {
                registry
                    .cleanup_macro_source_digests
                    .insert(path.clone(), value);
            }
            for name in NAMES {
                assert!(
                    !core_cmp(&["core".into(), "cmp".into()], name, Some(&registry)),
                    "{path}: {name}"
                );
            }
        }
    }
}

#[test]
fn comparison_imports_preserve_historical_and_original_scope_boundaries() {
    let source = "struct Error(()); use core::cmp::{Eq, Ord, PartialEq, PartialOrd}; #[derive(Eq, Ord, PartialEq, PartialOrd)] struct Value;";
    assert!(super::super::closure::inspect_external_source(source, true).is_ok());
    assert!(super::super::closure::inspect_external_source(source, false).is_err());
    for extra in [
        "mod core {} use core::cmp::Ord;",
        "use external::cmp::Ord;",
        "use core::cmp::Ord as PartialEq;",
        "use core::cmp::Ord as Ord;",
        "use core::cmp::*; #[derive(Ord)] struct Value;",
        "use core::cmp::Ord; impl Drop for Error { fn drop(&mut self) {} }",
        "fn nested() { use core::cmp::Ord; impl Drop for Error { fn drop(&mut self) {} } }",
        "use core::cmp::Ord; macro_rules! hidden { () => { impl Drop for Error { fn drop(&mut self) {} } }; } hidden!();",
    ] {
        let source = format!("struct Error(()); {extra}");
        assert!(
            super::super::closure::inspect_external_source(&source, true).is_err(),
            "{source}"
        );
    }
}
